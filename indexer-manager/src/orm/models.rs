// Generated by diesel_ext
#![allow(unused)]
#![allow(clippy::all)]
use super::schema::indexers;
use crate::orm::IndexerStatus;
use diesel::insert_into;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Clone, Default, Debug, Identifiable, Serialize, Insertable, Deserialize)]
#[primary_key(v_id)]
pub struct Indexer {
    pub network: Option<String>,
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub repository: Option<String>,
    pub manifest: String,
    pub mapping: String,
    pub unpack_instruction: String,
    pub graphql: String,
    pub status: IndexerStatus,
    pub deleted: bool,
    pub address: Option<String>,
    pub start_block: i64,
    pub got_block: i64,
    pub version: Option<String>,
    pub hash: String,
    pub v_id: i64,
}
