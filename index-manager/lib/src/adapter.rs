use crate::ipfs::read_config_file;
use crate::type_index::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;

pub async fn adapter_init(index_config: &IndexConfig) -> Result<(), Box<dyn Error>> {
    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    let config_value = read_config_file(&index_config.config);

    log::info!("Load library from {:?}", &index_config.mapping);
    let mut adapter = AdapterManager::new();
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &index_config.mapping,
        )
        .await;
    Ok(())
}
