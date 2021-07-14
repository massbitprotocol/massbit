use std::{path::PathBuf, env};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Read};
use tokio_compat_02::FutureExt;
use tonic::{Request};
use diesel::{RunQueryDsl, PgConnection, Connection, Queryable, QueryResult};
use diesel::result::DatabaseErrorInformation;
use reqwest::Client;
use serde_json::json;
use postgres::{Connection as PostgreConnection, TlsMode};
use serde::{Deserialize};
use node_template_runtime::Event;
use sp_core::{sr25519, H256 as Hash};
<<<<<<< HEAD
use lazy_static::lazy_static;
=======
>>>>>>> main

// Massbit dependencies
use ipfs_client::core::create_ipfs_clients;
use plugin::manager::PluginManager;
use stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
use crate::types::{DeployParams, DeployType, Indexer, DetailParams};
use index_store::core::IndexStore;
use massbit_chain_substrate::data_type::{SubstrateBlock as Block, SubstrateHeader as Header, SubstrateUncheckedExtrinsic as Extrinsic, decode_transactions, decode, SubstrateBlock};

// Configs
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
<<<<<<< HEAD

lazy_static! {
    static ref CHAIN_READER_URL: String = env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref HASURA_URL: String = env::var("HASURA_URL").unwrap_or(String::from("http://localhost:8080/v1/query"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING").unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String = env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

type EventRecord = system::EventRecord<Event, Hash>;

pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
=======
const URL: &str = "http://127.0.0.1:50051";
const CONNECTION_STRING: &'static str = "postgres://graph-node:let-me-in@localhost";
const HASURA: &'static str = "http://localhost:8080/v1/query";
type EventRecord = system::EventRecord<Event, Hash>;

pub async fn get_index_config(ipfs_config_hash: &String) -> serde_yaml::Mapping {
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
>>>>>>> main
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    serde_yaml::from_slice(&file_bytes).unwrap()
}

pub async fn get_mapping_file_from_ipfs(ipfs_mapping_hash: &String) -> String {
<<<<<<< HEAD
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
=======
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
>>>>>>> main
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_mapping_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = [ipfs_mapping_hash, ".so"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!("[Index Manager Helper] Write SO file to local storage successfully");
            file_name
        },
        Err(err) => {
            panic!("[Index Manager Helper] Could not write file to local storage {:#?}", err)
        }
    }
}

pub async fn get_config_file_from_ipfs(ipfs_config_hash: &String) -> String {
<<<<<<< HEAD
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
=======
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
>>>>>>> main
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await; // Refactor to use lazy load

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_config_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_name = [ipfs_config_hash, ".yaml"].join("");
    let res = fs::write(&file_name, file_bytes); // Add logger and says that write file successfully

    match res {
        Ok(_) => {
            log::info!("[Index Manager Helper] Write project.yaml file to local storage successfully");
            file_name
        },
        Err(err) => {
            panic!("[Index Manager Helper] Could not write file to local storage {:#?}", err)
        }
    }
}

pub async fn get_raw_query_from_ipfs(ipfs_model_hash: &String) -> String {
    log::info!("[Index Manager Helper] Downloading Raw Query from IPFS");
<<<<<<< HEAD
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
=======
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
>>>>>>> main
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    let file_bytes = ipfs_clients[0]
        .cat_all(ipfs_model_hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let raw_query = std::str::from_utf8(&file_bytes).unwrap();
    String::from(raw_query)
}

pub fn get_mapping_file_from_local(mapping_path: &String) -> PathBuf {
    let so_file_path = PathBuf::from(mapping_path.to_string());
    so_file_path
}

pub fn get_config_file_from_local(config_path: &String) -> String {
    let mut config_file = String::new();
    let mut f = File::open(config_path).expect("Unable to open file");
    f.read_to_string(&mut config_file).expect("Unable to read string");
    config_file
}

pub fn get_raw_query_from_local(model_path: &String) -> String {
    let mut raw_query = String::new();
    let mut f = File::open(model_path).expect("Unable to open file");
    f.read_to_string(&mut raw_query).expect("Unable to read string");
    raw_query
}

pub fn create_new_indexer_detail_table(connection: &PgConnection, raw_query: &String) {
    let query = diesel::sql_query(raw_query.clone());
    println!("Running: {}", raw_query);
    query.execute(connection);
}

pub fn create_indexers_table_if_not_exists(connection: &PgConnection) {
    let mut query = String::new();
    let mut f = File::open("./indexer/migration/indexers.sql").expect("Unable to open file");
    f.read_to_string(&mut query).expect("Unable to read string"); // Get raw query
    let result = diesel::sql_query(query).execute(connection);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] Init table Indexer");
        },
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };
}

pub fn read_config_file(config_file_path: &String) -> serde_yaml::Value{
    let mut project_config_string = String::new();
    let mut f = File::open(config_file_path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    f.read_to_string(&mut project_config_string).expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    project_config
}

pub fn insert_new_indexer(connection: &PgConnection, id: &String, project_config: serde_yaml::Value) {
    let network = project_config["dataSources"][0]["kind"].as_str().unwrap();
    let name = project_config["dataSources"][0]["name"].as_str().unwrap();

    let add_new_indexer = format!("INSERT INTO indexers(id, name, network) VALUES ('{}','{}','{}');", id, name, network);
    let result = diesel::sql_query(add_new_indexer).execute(connection);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Helper] New indexer created");
        },
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };
}

pub async fn track_hasura_table(table_name: &String) {
    let gist_body = json!({
        "type": "track_table",
        "args": {
            "schema": "public",
            "name": table_name.to_lowercase(),
        }
    });
    Client::new()
<<<<<<< HEAD
        .post(&*HASURA_URL)
=======
        .post(HASURA)
>>>>>>> main
        .json(&gist_body)
        .send().compat().await.unwrap();
}

pub async fn loop_blocks(params: DeployParams) -> Result<(), Box<dyn Error>> {
<<<<<<< HEAD
    let store = IndexStore {
        connection_string: DATABASE_CONNECTION_STRING.to_string(),
=======
    // Init Store
    let db_connection_string = match env::var("DATABASE_URL") {
        Ok(connection) => connection,
        Err(_) => String::from("postgres://graph-node:let-me-in@localhost")
    };
    let store = IndexStore {
        connection_string: db_connection_string,
>>>>>>> main
    };

    // Get mapping file, raw query to create new table and project.yaml config
    let (mapping_file_path, raw_query, config_file_path) = match params.deploy_type {
        DeployType::Local => {
            let raw_query = get_raw_query_from_local(&params.model_path);
            let mapping_file_path = get_mapping_file_from_local(&params.mapping_path);
            let config_file_path = get_config_file_from_local(&params.config_path);
            (mapping_file_path, raw_query, config_file_path)
        },
        DeployType::Ipfs => {
            let raw_query = get_raw_query_from_ipfs(&params.model_path).await;

            let mapping_file_name = get_mapping_file_from_ipfs(&params.mapping_path).await;
            let mapping_file_location = ["./", &mapping_file_name].join("");
            let mapping_file_path = PathBuf::from(mapping_file_location.to_string());

            let config_file_path = get_config_file_from_ipfs(&params.config_path).await;
            (mapping_file_path, raw_query, config_file_path)
        },
    };

<<<<<<< HEAD
    let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!("Error connecting to {}", *DATABASE_CONNECTION_STRING));
=======
    let connection = PgConnection::establish(CONNECTION_STRING).expect(&format!("Error connecting to {}", CONNECTION_STRING));
>>>>>>> main
    create_new_indexer_detail_table(&connection, &raw_query);

    // Track the newly created table with hasura
    track_hasura_table(&params.table_name).await;

    // Create indexers table so we can keep track of the indexers status
    create_indexers_table_if_not_exists(&connection);

    // Read project.yaml config and add a new indexer row
    let project_config = read_config_file(&config_file_path);
    insert_new_indexer(&connection, &params.index_name, project_config);

    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
<<<<<<< HEAD
    let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await.unwrap();
=======
    let mut client = StreamoutClient::connect(URL).await.unwrap();
>>>>>>> main
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    // Subscribe new blocks
    log::info!("[Index Manager Helper] Start plugin manager");
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        log::info!("[Index Manager Helper] Received block = {:?}, hash = {:?} from {:?}",data.block_number, data.block_hash, params.index_name);
        let mut plugins = PluginManager::new(&store);
        unsafe {
            plugins.load(mapping_file_path.clone()).unwrap();
        }

        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: Block = decode(&mut data.payload).unwrap();
                println!("Received BLOCK: {:?}", block.header.number);
                plugins.handle_block("test", &block);
            },
            Some(DataType::Event) => {
                let event: EventRecord = decode(&mut data.payload).unwrap();
                println!("Received EVENT: {:?}", event);
            },
            Some(DataType::Transaction) => {
                let extrinsics: Vec<Extrinsic> = decode_transactions(&mut data.payload).unwrap();
                println!("Received Extrinsic: {:?}", extrinsics);
            },

            _ => {
                println!("Not support data type: {:?}", &data.data_type);
            }
        }
    }
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    let mut client =
<<<<<<< HEAD
        PostgreConnection::connect(DATABASE_CONNECTION_STRING.clone(), TlsMode::None).unwrap();

    // TODO check for deploy success or not
    // TODO: add check if table does not exists
=======
        PostgreConnection::connect("postgresql://graph-node:let-me-in@localhost:5432/graph-node", TlsMode::None).unwrap();

>>>>>>> main
    let mut indexers: Vec<Indexer> = Vec::new();
    for row in &client.query("SELECT id, network, name FROM indexers", &[]).unwrap() {
        let indexer = Indexer {
            id: row.get(0),
            network: row.get(1),
            name: row.get(2),
        };
        indexers.push(indexer);
    }
    Ok((indexers))
}

// Comment this function until we have implemented v2 so we'll have data in the indexed detail table
// Query the indexed data (detail)
// pub async fn detail_handler_helper(params: DetailParams) -> Result<Vec<String>, Box<dyn Error>> {
//     let mut client =
//         PostgreConnection::connect("postgresql://graph-node:let-me-in@localhost:5432/graph-node", TlsMode::None).unwrap();
//     let mut indexers: Vec<Indexer> = Vec::new();
//     let mut indexers_clone: Vec<Indexer> = Vec::new();
//     for row in &client.query("SELECT id, network, name, index_data_ref FROM indexers WHERE id=$1 LIMIT 1", &[&params.index_name]).unwrap() {
//         let indexer = Indexer {
//             id: row.get(0),
//             network: row.get(1),
//             name: row.get(2),
//             index_data_ref: row.get(3),
//         };
//         indexers.push(indexer);
//     }
//
//     let index_data_ref = indexers.into_iter().nth(0).unwrap().index_data_ref;
//     let select_all_index_data_query = format!("SELECT * FROM {}", index_data_ref);
//     let rows = &client.query(&select_all_index_data_query, &[]).unwrap();
//
//     let mut temp: String = "".to_string();
//     let mut data: Vec<String> = Vec::new();
//     for (rowIndex, row) in rows.iter().enumerate() {
//         for (colIndex, column) in row.columns().iter().enumerate() {
//             let colType: String = column.type_().to_string();
//
//             if colType == "int4" { //i32
//                 let value: i32 = row.get(colIndex);
//                 temp = format!("{{ '{}':'{}' }}", column.name(), value.to_string());
//                 data.push(temp);
//             }
//             else if colType == "text" {
//             }
//             //TODO: more type support
//             else {
//                 //TODO: raise error
//             }
//         }
//     }
//
//     Ok((data))
// }