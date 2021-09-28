use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::{
    pg::Pg,
    serialize::Output,
    sql_types::Text,
    types::{FromSql, ToSql},
    PgConnection,
};
use massbit::{
    components::store::{self, DeploymentLocator, EntityType, WritableStore as WritableStoreTrait},
    prelude::{IndexerStore as IndexerStoreTrait, *},
    util::timed_cache::TimedCache,
};
use std::{collections::BTreeMap, collections::HashMap, sync::Arc};
use std::{fmt, io::Write};
use std::{iter::FromIterator, time::Duration};

use crate::deployment_store::DeploymentStore;
use crate::{
    connection_pool::ConnectionPool,
    primary,
    primary::{DeploymentId, Site},
    relational::Layout,
};

/// The name of a database shard; valid names must match `[a-z0-9_]+`
#[derive(Clone, Debug, Eq, PartialEq, Hash, AsExpression, FromSqlRow)]
pub struct Shard(String);

lazy_static! {
    /// The name of the primary shard that contains all instance-wide data
    pub static ref PRIMARY_SHARD: Shard = Shard("primary".to_string());
}

/// How long to cache information about a deployment site
const SITES_CACHE_TTL: Duration = Duration::from_secs(120);

impl Shard {
    pub fn new(name: String) -> Result<Self, StoreError> {
        if name.is_empty() {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must not be empty"
            )));
        }
        if name.len() > 30 {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names can be at most 30 characters, but `{}` has {} characters",
                name,
                name.len()
            )));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must only contain lowercase alphanumeric characters or '_'"
            )));
        }
        Ok(Shard(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Shard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl FromSql<Text, Pg> for Shard {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Shard::new(s).map_err(Into::into)
    }
}

impl ToSql<Text, Pg> for Shard {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        <String as ToSql<Text, Pg>>::to_sql(&self.0, out)
    }
}

#[derive(Clone)]
pub struct IndexerStore {
    inner: Arc<IndexerStoreInner>,
}

impl IndexerStore {
    pub fn new(stores: Vec<(Shard, ConnectionPool, Vec<ConnectionPool>, Vec<usize>)>) -> Self {
        Self {
            inner: Arc::new(IndexerStoreInner::new(stores)),
        }
    }
}

impl std::ops::Deref for IndexerStore {
    type Target = IndexerStoreInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct IndexerStoreInner {
    primary: ConnectionPool,
    stores: HashMap<Shard, Arc<DeploymentStore>>,
    /// Cache for the mapping from deployment id to shard/namespace/id. Only
    /// active sites are cached here to ensure we have a unique mapping from
    /// `SubgraphDeploymentId` to `Site`. The cache keeps entry only for
    /// `SITES_CACHE_TTL` so that changes, in particular, activation of a
    /// different deployment for the same hash propagate across different
    /// graph-node processes over time.
    sites: TimedCache<DeploymentHash, Site>,
}

impl IndexerStoreInner {
    pub fn new(stores: Vec<(Shard, ConnectionPool, Vec<ConnectionPool>, Vec<usize>)>) -> Self {
        let primary = stores
            .iter()
            .find(|(name, _, _, _)| name == &*PRIMARY_SHARD)
            .map(|(_, pool, _, _)| pool.clone())
            .expect("we always have a primary shard");
        let stores = HashMap::from_iter(stores.into_iter().map(
            |(name, main_pool, read_only_pools, weights)| {
                (
                    name,
                    Arc::new(DeploymentStore::new(main_pool, read_only_pools, weights)),
                )
            },
        ));
        let sites = TimedCache::new(SITES_CACHE_TTL);
        IndexerStoreInner {
            primary,
            stores,
            sites,
        }
    }

    fn find_site(&self, id: DeploymentId) -> Result<Arc<Site>, StoreError> {
        if let Some(site) = self.sites.find(|site| site.id == id) {
            return Ok(site);
        }

        let conn = self.primary_conn()?;
        let site = conn
            .find_site_by_ref(id)?
            .ok_or_else(|| StoreError::DeploymentNotFound(id.to_string()))?;
        let site = Arc::new(site);

        self.cache_active(&site);
        Ok(site)
    }

    /// Get a connection to the primary shard. Code must never hold one of these
    /// connections while also accessing a `DeploymentStore`, since both
    /// might draw connections from the same pool, and trying to get two
    /// connections can deadlock the entire process if the pool runs out
    /// of connections in between getting the first one and trying to get the
    /// second one.
    fn primary_conn(&self) -> Result<primary::Connection, StoreError> {
        let conn = self.primary.get_with_timeout_warning()?;
        Ok(primary::Connection::new(conn))
    }

    fn cache_active(&self, site: &Arc<Site>) {
        if site.active {
            self.sites.set(site.deployment.clone(), site.clone());
        }
    }

    /// Return the active `Site` for this deployment hash
    fn site(&self, id: &DeploymentHash) -> Result<Arc<Site>, StoreError> {
        if let Some(site) = self.sites.get(id) {
            return Ok(site);
        }

        let conn = self.primary_conn()?;
        let site = conn
            .find_active_site(id)?
            .ok_or_else(|| StoreError::DeploymentNotFound(id.to_string()))?;
        let site = Arc::new(site);

        self.cache_active(&site);
        Ok(site)
    }

    /// Return the store and site for the active deployment of this
    /// deployment hash
    fn store(&self, id: &DeploymentHash) -> Result<(&Arc<DeploymentStore>, Arc<Site>), StoreError> {
        let site = self.site(id)?;
        let store = self
            .stores
            .get(&site.shard)
            .ok_or(StoreError::UnknownShard(site.shard.as_str().to_string()))?;
        Ok((store, site))
    }

    fn for_site(&self, site: &Site) -> Result<&Arc<DeploymentStore>, StoreError> {
        self.stores
            .get(&site.shard)
            .ok_or(StoreError::UnknownShard(site.shard.as_str().to_string()))
    }

    fn layout(&self, id: &DeploymentHash) -> Result<Arc<Layout>, StoreError> {
        let (store, site) = self.store(id)?;
        store.find_layout(site)
    }
}

#[async_trait::async_trait]
impl IndexerStoreTrait for IndexerStore {
    fn writable(
        &self,
        deployment: &DeploymentLocator,
    ) -> Result<Arc<dyn store::WritableStore>, StoreError> {
        let site = self.find_site(deployment.id.into())?;
        Ok(Arc::new(WritableStore::new(self.clone(), site)?))
    }

    fn create_indexer(&self, name: IndexerName) -> Result<String, StoreError> {
        let pconn = self.primary_conn()?;
        pconn.transaction(|| pconn.create_indexer(&name))
    }

    fn input_schema(&self, indexer_id: &DeploymentHash) -> Result<Arc<Schema>, StoreError> {
        todo!()
    }
}

/// A wrapper around `SubgraphStore` that only exposes functions that are
/// safe to call from `WritableStore`, i.e., functions that either do not
/// deal with anything that depends on a specific deployment
/// location/instance, or where the result is independent of the deployment
/// instance
struct WritableIndexerStore(IndexerStore);

impl WritableIndexerStore {
    fn primary_conn(&self) -> Result<primary::Connection, StoreError> {
        self.0.primary_conn()
    }

    fn layout(&self, id: &DeploymentHash) -> Result<Arc<Layout>, StoreError> {
        self.0.layout(id)
    }
}

struct WritableStore {
    store: WritableIndexerStore,
    writable: Arc<DeploymentStore>,
    site: Arc<Site>,
}

impl WritableStore {
    fn new(indexer_store: IndexerStore, site: Arc<Site>) -> Result<Self, StoreError> {
        let store = WritableIndexerStore(indexer_store.clone());
        let writable = indexer_store.for_site(site.as_ref())?.clone();
        Ok(Self {
            store,
            writable,
            site,
        })
    }
}
