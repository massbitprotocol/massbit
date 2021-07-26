use std::path::PathBuf;

// Massbit dependencies
use crate::types::{IndexConfig};
use crate::config_helper::{get_query_ipfs, get_mapping_ipfs, get_config_ipfs, get_query_local, get_config_local, get_mapping_local, read_config_file, get_schema_ipfs, get_schema_local};
use serde_yaml::Value;

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

/*********************
* Index Config Local *
*********************/
pub struct IndexConfigLocalBuilder {
    schema: String,
    config: String,
    mapping: PathBuf,
    query: String,
}

impl Default for IndexConfigLocalBuilder {
    fn default() -> IndexConfigLocalBuilder {
        IndexConfigLocalBuilder {
            schema: "".to_string(),
            config: Default::default(),
            mapping: "".to_string().parse().unwrap(),
            query: "".to_string(),
        }
    }
}

impl IndexConfigLocalBuilder {
    pub fn query(mut self, query: String) -> IndexConfigLocalBuilder {
        self.query = get_query_local(&query);
        self
    }

    pub fn mapping(mut self, mapping: String) -> IndexConfigLocalBuilder {
        self.mapping = get_mapping_local(&mapping);
        self
    }

    pub fn config(mut self, config: String) -> IndexConfigLocalBuilder {
        self.config = get_config_local(&config);
        self
    }

    pub fn schema(mut self, schema: String) -> IndexConfigLocalBuilder {
        self.schema = get_schema_local(&schema);
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            query: self.query,
        }
    }
}

/********************
* Index Config IPFS *
********************/
pub struct IndexConfigIpfsBuilder {
    schema: String,
    config: String,
    mapping: PathBuf,
    query: String,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            schema: "".to_string(),
            config: Default::default(),
            mapping: "".to_string().parse().unwrap(),
            query: "".to_string(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    pub async fn query(mut self, query: String) -> IndexConfigIpfsBuilder {
        self.query = get_query_ipfs(&query).await;
        self
    }

    pub async fn mapping(mut self, mapping: String) -> IndexConfigIpfsBuilder {
        let mapping_name = get_mapping_ipfs(&mapping).await;
        let mapping_file = ["./", &mapping_name].join("");
        self.mapping = PathBuf::from(mapping_file.to_string());
        self
    }

    pub async fn config(mut self, config: String) -> IndexConfigIpfsBuilder {
        self.config = get_config_ipfs(&config).await;
        self
    }

    pub async fn schema(mut self, schema: String) -> IndexConfigIpfsBuilder {
        self.schema = get_schema_ipfs(&schema).await;
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            query: self.query,
        }
    }
}
