use super::loader::load_dynamic_data_sources;
use super::IndexerInstance;
use atomic_refcell::AtomicRefCell;
use fail::fail_point;
use lazy_static::lazy_static;
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::blockchain::{BlockchainKind, DataSource};
use massbit::blockchain::{TriggerData, TriggersAdapter};
use massbit::components::indexer::DataSourceTemplateInfo;
use massbit::components::store::{IndexerStore, WritableStore};
use massbit::data::store::scalar::Bytes;
use massbit::ext::futures::{CancelHandle, FutureExtension};
use massbit::prelude::TryStreamExt;
use massbit::prelude::{IndexerInstanceManager as IndexerInstanceManagerTrait, *};
use massbit::util::lfu_cache::LfuCache;
use massbit::{
    blockchain::{block_stream::BlockStreamEvent, Blockchain, TriggerFilter as _},
    components::indexer::MappingError,
};
use massbit::{
    blockchain::{Block, BlockchainMap},
    components::store::{DeploymentId, DeploymentLocator, ModificationsAndCache},
};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::task;

lazy_static! {
    /// Size limit of the entity LFU cache, in bytes.
    // Multiplied by 1000 because the env var is in KB.
    pub static ref ENTITY_CACHE_SIZE: usize = 1000
        * std::env::var("GRAPH_ENTITY_CACHE_SIZE")
            .unwrap_or("10000".into())
            .parse::<usize>()
            .expect("invalid GRAPH_ENTITY_CACHE_SIZE");

    // Keep deterministic errors non-fatal even if the indexer is pending.
    // Used for testing Graph Node itself.
    pub static ref DISABLE_FAIL_FAST: bool =
        std::env::var("GRAPH_DISABLE_FAIL_FAST").is_ok();
}

type SharedInstanceKeepAliveMap = Arc<RwLock<HashMap<DeploymentId, CancelGuard>>>;

struct IndexingInputs<C: Blockchain> {
    deployment: DeploymentLocator,
    start_blocks: Vec<BlockNumber>,
    store: Arc<dyn WritableStore>,
    triggers_adapter: Arc<C::TriggersAdapter>,
    chain: Arc<C>,
    templates: Arc<Vec<C::DataSourceTemplate>>,
}

struct IndexingState<T: RuntimeHostBuilder<C>, C: Blockchain> {
    instance: IndexerInstance<C, T>,
    instances: SharedInstanceKeepAliveMap,
    filter: C::TriggerFilter,
    entity_lfu_cache: LfuCache<EntityKey, Option<Entity>>,
}

struct IndexingContext<T: RuntimeHostBuilder<C>, C: Blockchain> {
    /// Read only inputs that are needed while indexing a indexer.
    pub inputs: IndexingInputs<C>,

    /// Mutable state that may be modified while indexing a indexer.
    pub state: IndexingState<T, C>,
}

pub struct IndexerInstanceManager<S, L> {
    indexer_store: Arc<S>,
    chains: Arc<BlockchainMap>,
    instances: SharedInstanceKeepAliveMap,
    link_resolver: Arc<L>,
}

#[async_trait]
impl<S, L> IndexerInstanceManagerTrait for IndexerInstanceManager<S, L>
where
    S: IndexerStore,
    L: LinkResolver + Clone,
{
    async fn start_indexer(self: Arc<Self>, loc: DeploymentLocator, manifest: serde_yaml::Mapping) {
        let instance_manager = self.cheap_clone();

        let indexer_start_future = async move {
            match BlockchainKind::from_manifest(&manifest)? {
                BlockchainKind::Ethereum => {
                    instance_manager
                        .start_indexer_inner::<chain_ethereum::Chain>(loc, manifest)
                        .await
                }
            }
        };

        massbit::spawn(async move {
            match indexer_start_future.await {
                Ok(()) => {}
                Err(err) => error!("Failed to start indexer, error: {}", format!("{}", err)),
            }
        });
    }
}

impl<S, L> IndexerInstanceManager<S, L>
where
    S: IndexerStore,
    L: LinkResolver + Clone,
{
    pub fn new(indexer_store: Arc<S>, chains: Arc<BlockchainMap>, link_resolver: Arc<L>) -> Self {
        IndexerInstanceManager {
            indexer_store,
            chains,
            instances: SharedInstanceKeepAliveMap::default(),
            link_resolver,
        }
    }

    async fn start_indexer_inner<C: Blockchain>(
        self: Arc<Self>,
        deployment: DeploymentLocator,
        manifest: serde_yaml::Mapping,
    ) -> Result<(), Error> {
        let indexer_store = self.indexer_store.cheap_clone();
        let store = self.indexer_store.writable(&deployment)?;

        let manifest: IndexerManifest<C> = {
            info!("Resolve indexer files using IPFS");

            let mut manifest = IndexerManifest::resolve_from_raw(
                deployment.hash.cheap_clone(),
                manifest,
                // Allow for infinite retries for indexer definition files.
                &self.link_resolver.as_ref().clone().with_retries(),
            )
            .await
            .context("Failed to resolve indexer from IPFS")?;

            let data_sources =
                load_dynamic_data_sources::<C>(store.clone(), manifest.templates.clone())
                    .await
                    .context("Failed to load dynamic data sources")?;

            info!("Successfully resolved indexer files using IPFS");

            // Add dynamic data sources to the indexer
            manifest.data_sources.extend(data_sources);

            info!(
                "Data source count at start: {}",
                manifest.data_sources.len()
            );

            manifest
        };

        let network = manifest.network_name();

        let chain = self
            .chains
            .get::<C>(network.clone())
            .with_context(|| format!("no chain configured for network {}", network))?
            .clone();

        // Obtain filters from the manifest
        let filter = C::TriggerFilter::from_data_sources(manifest.data_sources.iter());
        let start_blocks = manifest.start_blocks();

        let templates = Arc::new(manifest.templates.clone());

        let triggers_adapter = chain
            .triggers_adapter()
            .map_err(|e| anyhow!("expected triggers adapter that matches deployment"))?
            .clone();

        let host_builder = runtime_wasm::RuntimeHostBuilder::new(
            chain.runtime_adapter(),
            self.link_resolver.cheap_clone(),
            indexer_store,
        );

        let instance = IndexerInstance::from_manifest(manifest, host_builder)?;

        // The indexer state tracks the state of the indexer instance over time
        let ctx = IndexingContext {
            inputs: IndexingInputs {
                deployment: deployment.clone(),
                start_blocks,
                store,
                triggers_adapter,
                chain,
                templates,
            },
            state: IndexingState {
                instance,
                instances: self.instances.cheap_clone(),
                filter,
                entity_lfu_cache: LfuCache::new(),
            },
        };

        // Keep restarting the indexer until it terminates. The indexer
        // will usually only run once, but is restarted whenever a block
        // creates dynamic data sources. This allows us to recreate the
        // block stream and include events for the new data sources going
        // forward; this is easier than updating the existing block stream.
        //
        // This is a long-running and unfortunately a blocking future (see #905), so it is run in
        // its own thread. It is also run with `task::unconstrained` because we have seen deadlocks
        // occur without it, possibly caused by our use of legacy futures and tokio versions in the
        // codebase and dependencies, which may not play well with the tokio 1.0 cooperative
        // scheduling. It is also logical in terms of performance to run this with `unconstrained`,
        // it has a dedicated OS thread so the OS will handle the preemption. See
        // https://github.com/tokio-rs/tokio/issues/3493.
        massbit::spawn_thread(deployment.to_string(), move || {
            if let Err(e) = massbit::block_on(task::unconstrained(run_indexer(ctx))) {
                error!("Indexer instance failed to run: {}", format!("{:#}", e));
            }
        });

        Ok(())
    }
}

async fn run_indexer<T, C>(mut ctx: IndexingContext<T, C>) -> Result<(), Error>
where
    T: RuntimeHostBuilder<C>,
    C: Blockchain,
{
    // Clone a few things for different parts of the async processing
    let store_for_err = ctx.inputs.store.cheap_clone();
    let id_for_err = ctx.inputs.deployment.hash.clone();
    let mut first_run = true;

    loop {
        debug!("Starting or restarting indexer");

        let block_stream_canceler = CancelGuard::new();
        let block_stream_cancel_handle = block_stream_canceler.handle();
        let mut block_stream = ctx
            .inputs
            .chain
            .new_block_stream(
                ctx.inputs.deployment.clone(),
                ctx.inputs.start_blocks[0].clone(),
                Arc::new(ctx.state.filter.clone()),
            )
            .await?
            .map_err(CancelableError::Error)
            .cancelable(&block_stream_canceler, || Err(CancelableError::Cancel));

        // Keep the stream's cancel guard around to be able to shut it down
        // when the indexer deployment is unassigned
        ctx.state
            .instances
            .write()
            .unwrap()
            .insert(ctx.inputs.deployment.id, block_stream_canceler);

        // Process events from the stream as long as no restart is needed
        loop {
            let block = match block_stream.next().await {
                Some(Ok(BlockStreamEvent::ProcessBlock(block))) => block,
                // Log and drop the errors from the block_stream
                // The block stream will continue attempting to produce blocks
                Some(Err(e)) => {
                    continue;
                }
                None => unreachable!("The block stream stopped producing blocks"),
            };

            let block_ptr = block.ptr();
            let res = process_block(
                ctx.inputs.triggers_adapter.cheap_clone(),
                ctx,
                block_stream_cancel_handle.clone(),
                block,
            )
            .await;

            match res {
                Ok((c, needs_restart)) => {
                    ctx = c;

                    // Unfail the indexer if it was previously failed.
                    // As an optimization we check this only on the first run.
                    if first_run {
                        first_run = false;
                    }

                    if needs_restart {
                        // Cancel the stream for real
                        ctx.state
                            .instances
                            .write()
                            .unwrap()
                            .remove(&ctx.inputs.deployment.id);

                        // And restart the indexer
                        break;
                    }
                }
                Err(BlockProcessingError::Canceled) => {
                    return Ok(());
                }

                // Handle unexpected stream errors by marking the indexer as failed.
                Err(e) => {
                    let message = format!("{:#}", e).replace("\n", "\t");
                    let err = anyhow!("{}", message);
                    return Err(err);
                }
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum BlockProcessingError {
    #[error("{0:#}")]
    Unknown(Error),

    #[error("indexer stopped while processing triggers")]
    Canceled,
}

impl From<Error> for BlockProcessingError {
    fn from(e: Error) -> Self {
        BlockProcessingError::Unknown(e)
    }
}

/// Processes a block and returns the updated context and a boolean flag indicating
/// whether new dynamic data sources have been added to the indexer.
async fn process_block<T: RuntimeHostBuilder<C>, C: Blockchain>(
    triggers_adapter: Arc<C::TriggersAdapter>,
    mut ctx: IndexingContext<T, C>,
    block_stream_cancel_handle: CancelHandle,
    block: BlockWithTriggers<C>,
) -> Result<(IndexingContext<T, C>, bool), BlockProcessingError> {
    let triggers = block.trigger_data;
    let block = Arc::new(block.block);
    let block_ptr = block.ptr();

    if triggers.len() == 1 {
        info!("1 trigger found in this block for this indexer");
    } else if triggers.len() > 1 {
        info!(
            "{} triggers found in this block for this indexer",
            triggers.len()
        );
    }

    // Process events one after the other, passing in entity operations
    // collected previously to every new event being processed
    let mut block_state = match process_triggers(
        BlockState::new(
            ctx.inputs.store.clone(),
            std::mem::take(&mut ctx.state.entity_lfu_cache),
        ),
        &ctx.state.instance,
        &block,
        triggers,
    )
    .await
    {
        // Triggers processed with no errors or with only determinstic errors.
        Ok(block_state) => block_state,

        // Some form of unknown or non-deterministic error ocurred.
        Err(MappingError::Unknown(e)) => return Err(BlockProcessingError::Unknown(e)),
    };

    // If new data sources have been created, restart the indexer after this block.
    // This is necessary to re-create the block stream.
    let needs_restart = block_state.has_created_data_sources();

    // This loop will:
    // 1. Instantiate created data sources.
    // 2. Process those data sources for the current block.
    // Until no data sources are created or MAX_DATA_SOURCES is hit.

    // Note that this algorithm processes data sources spawned on the same block _breadth
    // first_ on the tree implied by the parent-child relationship between data sources. Only a
    // very contrived indexer would be able to observe this.
    while block_state.has_created_data_sources() {
        // Instantiate dynamic data sources, removing them from the block state.
        let (data_sources, runtime_hosts) =
            create_dynamic_data_sources(&mut ctx, block_state.drain_created_data_sources())?;

        let filter = C::TriggerFilter::from_data_sources(data_sources.iter());

        // Reprocess the triggers from this block that match the new data sources
        let block_with_triggers = triggers_adapter
            .triggers_in_block(block.as_ref().clone(), &filter)
            .await?;

        let triggers = block_with_triggers.trigger_data;

        if triggers.len() == 1 {
            info!("1 trigger found in this block for the new data sources");
        } else if triggers.len() > 1 {
            info!(
                "{} triggers found in this block for the new data sources",
                triggers.len()
            );
        }

        // Add entity operations for the new data sources to the block state
        // and add runtimes for the data sources to the indexer instance.
        persist_dynamic_data_sources(&mut ctx, &mut block_state.entity_cache, data_sources);

        // Process the triggers in each host in the same order the
        // corresponding data sources have been created.
        for trigger in triggers {
            block_state = IndexerInstance::<C, T>::process_trigger_in_runtime_hosts(
                &runtime_hosts,
                &block,
                &trigger,
                block_state,
            )
            .await
            .map_err(|e| match e {
                MappingError::Unknown(e) => BlockProcessingError::Unknown(e),
            })?;
        }
    }

    // Avoid writing to store if block stream has been canceled
    if block_stream_cancel_handle.is_canceled() {
        return Err(BlockProcessingError::Canceled);
    }

    let ModificationsAndCache {
        modifications: mods,
        data_sources,
        entity_lfu_cache: mut cache,
    } = block_state
        .entity_cache
        .as_modifications()
        .map_err(|e| BlockProcessingError::Unknown(e.into()))?;

    cache.evict(*ENTITY_CACHE_SIZE);

    // Put the cache back in the ctx, asserting that the placeholder cache was not used.
    assert!(ctx.state.entity_lfu_cache.is_empty());
    ctx.state.entity_lfu_cache = cache;

    if !mods.is_empty() {
        info!("Applying {} entity operation(s)", mods.len());
    }

    // Transact entity operations into the store and update the
    // indexer's block stream pointer
    let store = &ctx.inputs.store;

    match store.transact_block_operations(block_ptr, mods, data_sources) {
        Ok(_) => Ok((ctx, needs_restart)),
        Err(e) => Err(anyhow!("Error while processing block stream for a indexer: {}", e).into()),
    }
}

async fn process_triggers<C: Blockchain>(
    mut block_state: BlockState<C>,
    instance: &IndexerInstance<C, impl RuntimeHostBuilder<C>>,
    block: &Arc<C::Block>,
    triggers: Vec<C::TriggerData>,
) -> Result<BlockState<C>, MappingError> {
    for trigger in triggers.into_iter() {
        let start = Instant::now();
        block_state = instance
            .process_trigger(block, &trigger, block_state)
            .await
            .map_err(move |mut e| {
                let error_context = trigger.error_context();
                if !error_context.is_empty() {
                    e = e.context(error_context);
                }
                e.context("failed to process trigger".to_string())
            })?;
    }
    Ok(block_state)
}

fn create_dynamic_data_sources<T: RuntimeHostBuilder<C>, C: Blockchain>(
    ctx: &mut IndexingContext<T, C>,
    created_data_sources: Vec<DataSourceTemplateInfo<C>>,
) -> Result<(Vec<C::DataSource>, Vec<Arc<T::Host>>), Error> {
    let mut data_sources = vec![];
    let mut runtime_hosts = vec![];

    for info in created_data_sources {
        // Try to instantiate a data source from the template
        let data_source = C::DataSource::try_from(info)?;

        // Try to create a runtime host for the data source
        let host = ctx
            .state
            .instance
            .add_dynamic_data_source(data_source.clone(), ctx.inputs.templates.clone())?;

        match host {
            Some(host) => {
                data_sources.push(data_source);
                runtime_hosts.push(host);
            }
            None => {
                fail_point!("error_on_duplicate_ds", |_| Err(anyhow!("duplicate ds")));
            }
        }
    }

    Ok((data_sources, runtime_hosts))
}

fn persist_dynamic_data_sources<T: RuntimeHostBuilder<C>, C: Blockchain>(
    ctx: &mut IndexingContext<T, C>,
    entity_cache: &mut EntityCache,
    data_sources: Vec<C::DataSource>,
) {
    // Add entity operations to the block state in order to persist
    // the dynamic data sources
    for data_source in data_sources.iter() {
        entity_cache.add_data_source(data_source);
    }

    // Merge filters from data sources into the block stream builder
    ctx.state.filter.extend(data_sources.iter());
}
