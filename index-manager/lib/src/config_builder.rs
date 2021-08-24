/**
 *** Objective of this file, is to build the IndexConfig from the user's Index Request
 *** It will connect to IPFS to get the files and save them to storage
 **/
// Generic dependencies
use std::path::PathBuf;
use lazy_static::lazy_static;

// Massbit dependencies
use crate::config::{
    generate_mapping_name_and_type, generate_random_hash, get_index_name, get_mapping_language,
};
use crate::ipfs::{download_ipfs_file_by_hash, read_config_file};
use crate::type_index::{IndexConfig, IndexIdentifier, Abi};
use crate::type_request::{DeployAbi, DeployParams};
use std::fs;
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;

lazy_static! {
    static ref GENERATED_FOLDER: String = String::from("index-manager/generated");
}

/**
 *** Builder Pattern
 *** Real example: https://github.com/graphprotocol/rust-web3/blob/3aac17f719b99494793111fd00a4505fe4670ca2/src/types/log.rs#L103
 *** Advantages:
 ***  - Separates methods for building from other methods.
 ***  - Prevents proliferation of constructors
 ***  - Can be used for one-liner initialisation as well as more complex construction.
 *** Note:
 ***  - I think this is useful when there's too many complex check that needs to be done and we want to hide it from the main logic
 *** Reference: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
 **/
/*******************************************************************************
  IndexConfigIpfsBuilder

  Description:
  To build the index config based on the config from IPFS
*******************************************************************************/
pub struct IndexConfigIpfsBuilder {
    schema: PathBuf,
    config: PathBuf,
    mapping: PathBuf,
    abi: Vec<Abi>,
    hash: String,
    subgraph: PathBuf,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            schema: Default::default(),
            config: Default::default(),
            mapping: Default::default(),
            abi: Default::default(),
            hash: generate_random_hash(),
            subgraph: Default::default(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    // Call to IPFS and download mapping to local storage
    // Mapping file type is decided by the self.config value
    pub async fn mapping(mut self, mapping: &String) -> IndexConfigIpfsBuilder {
        assert_eq!(
            self.config.as_os_str().is_empty(),
            false,
            "Config should be provided before mapping and schema"
        );
        let config_value = read_config_file(&self.config);
        let file_name = generate_mapping_name_and_type(&config_value);
        self.mapping = download_ipfs_file_by_hash(&file_name, &self.hash, mapping).await;
        self
    }

    // Call to IPFS and download config to local storage
    pub async fn config(mut self, config: &String) -> IndexConfigIpfsBuilder {
        self.config = download_ipfs_file_by_hash(
            &String::from("project.yaml"),
            &self.hash,
            config,
        ).await;

        self
    }

    // Call to IPFS and download schema to local storage
    pub async fn schema(mut self, schema: &String) -> IndexConfigIpfsBuilder {
        self.schema = download_ipfs_file_by_hash(
            &String::from("schema.graphql"),
            &self.hash,
            schema,
        ).await;
        self
    }

    // Call to IPFS and download ABIs to local storage
    pub async fn abi(mut self, abi: Option<Vec<DeployAbi>>) -> IndexConfigIpfsBuilder {
        match abi {
            Some(v) => {
                self.abi = build_abi(v, &self.hash).await;
                self
            }
            None => {
                println!(".SO mapping or this index type doesn't support ABIs");
                self.abi = vec![];
                self
            },
        }
    }

    pub async fn generate_subgraph(mut self, params: &DeployParams) -> IndexConfigIpfsBuilder {
        assert_eq!(
            self.config.as_os_str().is_empty(),
            false,
            "Config should be provided before calling this function",
        );
        assert_eq!(
            self.schema.as_os_str().is_empty(),
            false,
            "Schema should be provided before calling this function"
        );
        // assert_eq!(
        //     self.abi.len(),
        //     0,
        //     "ABI should be provided before calling this function"
        // );

        // Map Schema
        // Write schema to file
        // let mut config_value = read_config_file(&self.config);


        let mut project_config_string = String::new();
        let mut f = File::open(&self.config).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
        f.read_to_string(&mut project_config_string)
            .expect("Unable to read string"); // Get raw query
        // let mut config_value: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
        let mut config_mapping: serde_yaml::Mapping = serde_yaml::from_str(&project_config_string).unwrap();
        // config_mapping.insert(Value::from("schema"), Value::from("b"));

        let mut fulltext_fields = BTreeMap::new();
        fulltext_fields.insert("name".to_string(), vec!["search".to_string()]);
        config_mapping.insert(Value::from("file"), Value::from("b"));


        // load_and_set_user_name(&mut module, "herobrine", "Brine-O");
        // let mut fulltext_entities = BTreeMap::new();
        // let mut fulltext_fields = BTreeMap::new();
        // fulltext_fields.insert("name".to_string(), vec!["search".to_string()]);
        // fulltext_entities.insert("User".to_string(), fulltext_fields);


        let content = serde_yaml::to_string(&config_mapping).unwrap();
        let file_path = [GENERATED_FOLDER.as_str(), &self.hash, "subgraph.yaml"].join("/");
        let res = fs::write(file_path.clone(), content);

        // let mut map = BTreeMap::new();
        // map.insert("x".to_string(), 1.0);
        // map.insert("y".to_string(), 2.0);
        // config_value["schema"]["file"] = serde_yaml::from_str("42").unwrap();
        // config_value["schema"]["file"][0] = Value::String(["/ipfs/", &params.schema].join(""));
        // let content = serde_yaml::to_string(&config_value).unwrap();
        // let file_path = [GENERATED_FOLDER.as_str(), &self.hash, "subgraph.yaml"].join("/");
        // let res = fs::write(file_path.clone(), content);

        // Map API (datasources / template)
        // TODO

        // Delete mapping / file
        // TODO

        self.subgraph = "".parse().unwrap();
        self
    }

    pub fn build(self) -> IndexConfig {
        let config = read_config_file(&self.config);
        let name = get_index_name(&config);

        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            abi: Option::Some(self.abi),
            subgraph: self.subgraph,
            identifier: IndexIdentifier {
                name: name.clone(),
                hash: self.hash.clone(),
                name_with_hash: format!("{}-{}", name, self.hash),
            },
        }
    }
}

/*******************************************************************************
  IndexConfigLocalBuilder

  Description:
  To build the index config based on the name and hash from the indexers table
*******************************************************************************/
impl Default for IndexConfigLocalBuilder {
    fn default() -> IndexConfigLocalBuilder {
        IndexConfigLocalBuilder {
            schema: Default::default(),
            config: Default::default(),
            mapping: Default::default(),
            hash: generate_random_hash(),
        }
    }
}

pub struct IndexConfigLocalBuilder {
    schema: PathBuf,
    config: PathBuf,
    mapping: PathBuf,
    hash: String,
}

impl IndexConfigLocalBuilder {
    pub async fn mapping(mut self, name: &String) -> IndexConfigLocalBuilder {
        let mapping = [GENERATED_FOLDER.as_str(), name, ".so"].join("");
        self.mapping = PathBuf::from(mapping.to_string());
        self
    }

    pub async fn config(mut self, name: &String) -> IndexConfigLocalBuilder {
        let config = [GENERATED_FOLDER.as_str(), name, ".yaml"].join("");
        self.config = PathBuf::from(config);
        self
    }

    pub async fn schema(mut self, name: &String) -> IndexConfigLocalBuilder {
        let schema = [GENERATED_FOLDER.as_str(), name, ".graphql"].join("");
        self.schema = PathBuf::from(schema);
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            // TODO: Add logic for these type so we can restart indexers if needed
            abi: Default::default(),
            subgraph: Default::default(),
            identifier: IndexIdentifier {
                // TODO: populate with the value from the indexer query result
                name: Default::default(),
                hash: Default::default(),
                name_with_hash: Default::default(),
            },
        }
    }
}


/******** Helper Functions **********/
// Build a new ABI struct from DeployABI
// Call to IPFS to and save the ABI files to local storage
async fn build_abi(abi_list: Vec<DeployAbi>, folder_name: &String) -> Vec<Abi>{
    let mut new_abi_list: Vec<Abi> = vec![];
    for deploy_abi in abi_list {
        let abi = Abi {
            name: deploy_abi.name.clone(),
            path: download_ipfs_file_by_hash(&deploy_abi.name, folder_name, &deploy_abi.hash).await
        };
        new_abi_list.push(abi);
    }
    new_abi_list
}