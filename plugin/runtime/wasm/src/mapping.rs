use crate::host_exports::HostExports;
use graph::blockchain::{Blockchain, HostFn, HostFnCtx};
use graph::components::subgraph::BlockState;
use graph::prelude::{BlockPtr, CheapClone, Logger};
use graph::runtime::AscHeap;
use graph_runtime_wasm::ValidModule;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::anyhow::Error;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub trait FromFile {
    fn from_file(file_path: impl AsRef<Path>) -> Result<ValidModule, anyhow::Error>;
}
impl FromFile for ValidModule {
    fn from_file(file_path: impl AsRef<Path>) -> Result<ValidModule, Error> {
        let engine = create_wasm_engine()?;
        let module = wasmtime::Module::from_file(&engine, file_path)?;

        let mut import_name_to_modules: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Unwrap: Module linking is disabled.
        for (name, module) in module
            .imports()
            .map(|import| (import.name().unwrap(), import.module()))
        {
            import_name_to_modules
                .entry(name.to_string())
                .or_default()
                .push(module.to_string());
        }

        Ok(ValidModule {
            module,
            import_name_to_modules,
        })
    }
}

pub struct MappingContext<C: Blockchain> {
    pub logger: Logger,
    pub host_exports: Arc<HostExports<C>>,
    pub block_ptr: BlockPtr,
    pub state: BlockState<C>,
    pub host_fns: Arc<Vec<HostFn>>,
}

impl<C: Blockchain> MappingContext<C> {
    pub fn derive_with_empty_state(&self) -> Self {
        MappingContext {
            logger: self.logger.cheap_clone(),
            host_exports: self.host_exports.cheap_clone(),
            //state: IndexerState::new(self.state.entity_cache.store.clone(), Default::default()),
            block_ptr: self.block_ptr.cheap_clone(),
            state: BlockState::new(self.state.entity_cache.store.clone(), Default::default()),
            //proof_of_indexing: self.proof_of_indexing.cheap_clone(),
            host_fns: self.host_fns.cheap_clone(),
        }
    }
}

/*
pub struct MappingRequest<C: Blockchain> {
    pub(crate) ctx: MappingContext<C>,
    pub(crate) trigger: C::MappingTrigger,
    //pub(crate) result_sender: Sender<Result<BlockState<C>, MappingError>>,
}


/// A pre-processed and valid WASM module, ready to be started as a WasmModule.
pub struct ValidModule {
    pub module: wasmtime::Module,

    // A wasm import consists of a `module` and a `name`. AS will generate imports such that they
    // have `module` set to the name of the file it is imported from and `name` set to the imported
    // function name or `namespace.function` if inside a namespace. We'd rather not specify names of
    // source files, so we consider that the import `name` uniquely identifies an import. Still we
    // need to know the `module` to properly link it, so here we map import names to modules.
    //
    // AS now has an `@external("module", "name")` decorator which would make things cleaner, but
    // the ship has sailed.
    pub import_name_to_modules: BTreeMap<String, Vec<String>>,
}

impl ValidModule {
    /// Pre-process and validate the module from binary.
    pub fn from_binary(raw_module: &[u8]) -> Result<Self, anyhow::Error> {
        let engine = create_engine()?;
        let module = wasmtime::Module::from_binary(&engine, raw_module)?;
        let mut import_name_to_modules: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Unwrap: Module linking is disabled.
        for (name, module) in module
            .imports()
            .map(|import| (import.name().unwrap(), import.module()))
        {
            import_name_to_modules
                .entry(name.to_string())
                .or_default()
                .push(module.to_string());
        }

        Ok(ValidModule {
            module,
            import_name_to_modules,
        })
    }
    /// Pre-process and validate the module from binary.
    pub fn from_file(file_path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let engine = create_engine()?;
        let module = wasmtime::Module::from_file(&engine, file_path)?;

        let mut import_name_to_modules: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Unwrap: Module linking is disabled.
        for (name, module) in module
            .imports()
            .map(|import| (import.name().unwrap(), import.module()))
        {
            import_name_to_modules
                .entry(name.to_string())
                .or_default()
                .push(module.to_string());
        }

        Ok(ValidModule {
            module,
            import_name_to_modules,
        })
    }
}
*/
fn create_wasm_engine() -> Result<wasmtime::Engine, anyhow::Error> {
    // We currently use Cranelift as a compilation engine. Cranelift is an optimizing compiler,
    // but that should not cause determinism issues since it adheres to the Wasm spec. Still we
    // turn off optional optimizations to be conservative.
    let mut config = wasmtime::Config::new();
    config.strategy(wasmtime::Strategy::Cranelift).unwrap();
    config.interruptable(true); // For timeouts.
    config.cranelift_nan_canonicalization(true); // For NaN determinism.
    config.cranelift_opt_level(wasmtime::OptLevel::None);
    wasmtime::Engine::new(&config)
}
