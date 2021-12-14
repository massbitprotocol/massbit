// Generated by diesel_ext
#![allow(unused)]
#![allow(clippy::all)]
use super::schema::{deployments, indexers};
use serde_derive::Serialize;
#[derive(Queryable, Clone, Default, Debug, Identifiable, Serialize, Insertable)]
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
    pub graphql: String,
    pub status: Option<String>,
    pub deleted: bool,
    pub address: Option<String>,
    pub start_block: i64,
    pub got_block: i64,
    pub version: Option<String>,
    pub hash: String,
    pub v_id: i64,
}

#[derive(Queryable, Clone, Default, Debug, Identifiable, Serialize, Insertable)]
#[primary_key(id)]
pub struct Deployment {
    pub id: i32,
    pub network: Option<String>,
    pub schema: String,
    pub synced: bool,
    pub manifest: String,
    pub mapping: String,
    pub graphql: String,
    pub status: Option<String>,
    pub deleted: bool,
    pub address: Option<String>,
    pub start_block: i64,
    pub got_block: i64,
    pub version: Option<String>,
    pub hash: String,
}
