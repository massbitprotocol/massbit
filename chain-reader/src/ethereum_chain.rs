use crate::Transport;
use crate::CONFIG;
use massbit::firehose::bstream::{BlockResponse, ChainType};

use anyhow::Error;
use chain_ethereum::{Chain, TriggerFilter};
use futures::Future;
use lazy_static::lazy_static;
use log::{info, warn};
use massbit::blockchain::block_stream::{BlockStreamEvent, BlockWithTriggers};
use massbit::blockchain::{Block, Blockchain};
use massbit::prelude::*;
use massbit_common::NetworkType;
use std::error::Error as StdError;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tonic::Status;
use web3::{types::H256, Web3};

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Ethereum;
// const PULLING_INTERVAL: u64 = 200;
pub(crate) const USE_WEBSOCKET: bool = false;
// const BLOCK_BATCH_SIZE: u64 = 10;
// const RETRY_GET_BLOCK_LIMIT: u32 = 10;
// const GET_BLOCK_TIMEOUT_SEC: u64 = 60;

fn get_web3(network: &NetworkType) -> Arc<Web3<Transport>> {
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, network).unwrap();
    let websocket_url = config.ws.clone();
    let http_url = config.url.clone();

    let (transport_event_loop, transport) = match USE_WEBSOCKET {
        false => Transport::new_rpc(&http_url, Default::default()),
        true => Transport::new_ws(&websocket_url),
    };
    std::mem::forget(transport_event_loop);
    Arc::new(Web3::new(transport))
}

lazy_static! {
    pub static ref WEB3_ETH: Arc<Web3<Transport>> = get_web3(&"ethereum".to_string());
    pub static ref WEB3_BSC: Arc<Web3<Transport>> = get_web3(&"bsc".to_string());
    pub static ref WEB3_MATIC: Arc<Web3<Transport>> = get_web3(&"matic".to_string());
}

#[derive(Error, Debug)]
pub enum IngestorError {
    /// The Ethereum node does not know about this block for some reason, probably because it
    /// disappeared in a chain reorg.
    #[error("Block data unavailable, block was likely uncled (block hash = {0:?})")]
    BlockUnavailable(H256),

    /// An unexpected error occurred.
    #[error("Ingestor error: {0}")]
    Unknown(Error),
}

pub async fn loop_get_block(
    chan: mpsc::Sender<Result<BlockResponse, Status>>,
    start_block: &Option<u64>,
    chain: Arc<Chain>,
    filter: TriggerFilter,
) -> Result<(), Box<dyn StdError>> {
    info!("Start get block {:?}", CHAIN_TYPE);
    info!("Init Ethereum adapter");

    let web3 = chain.eth_adapters.adapters[0].adapter.web3.clone();

    let version = web3
        .net()
        .version()
        .wait()
        .unwrap_or("Cannot get version".to_string());

    // let filter = TriggerFilter::from_data_sources(Vec::new().iter());
    // let filter_json = serde_json::to_string(&filter)?;
    // let filter = serde_json::from_str(filter_json.as_str())?;
    let start_blocks = vec![start_block.unwrap() as i32];
    let mut block_stream = chain
        .new_block_stream(start_blocks[0], Arc::new(filter))
        .await?;
    loop {
        let block = match block_stream.next().await {
            Some(Ok(BlockStreamEvent::ProcessBlock(block))) => block,
            Some(Err(_)) => {
                continue;
            }
            None => unreachable!("The block stream stopped producing blocks"),
        };
        println!("{}", block.block.number());
        let block_hash = block.block.hash().to_string();
        let block_number = block.block.number() as u64;
        // Create generic block
        let generic_block = _create_generic_block_with_trigger(&block, version.clone());
        // Send data to GRPC stream
        if !chan.is_closed() {
            let send_res = chan.send(Ok(generic_block as BlockResponse)).await;
            if send_res.is_ok() {
                info!("gRPC successfully sending block {}", &block_number);
            } else {
                warn!("gRPC unsuccessfully sending block {}", &block_number);
            }
        } else {
            return Err("Stream is closed!".into());
        }
    }
}

fn _create_generic_block_with_trigger(
    block: &BlockWithTriggers<Chain>,
    version: String,
) -> BlockResponse {
    let generic_data = BlockResponse {
        version,
        payload: serde_json::to_vec(block).unwrap(),
    };
    // Deserialize
    // let decode_block: BlockWithTriggers<Chain> =
    //     serde_json::from_slice(&generic_data.payload).unwrap();
    // println!("{:?}", decode_block);

    generic_data
}
