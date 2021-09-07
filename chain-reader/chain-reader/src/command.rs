use crate::ethereum_chain;
use crate::grpc_stream::StreamService;
use crate::solana_chain;
use crate::substrate_chain;
use crate::{
    grpc_stream::stream_mod::{streamout_server::StreamoutServer, ChainType},
    CONFIG,
};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tonic::transport::Server;

#[derive(Clone, Debug)]
pub struct Config {
    pub chains: HashMap<ChainType, ChainConfig>,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub url: String,
    pub ws: String,
    pub start_block: Option<u64>,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Broadcast Channel
    let mut chans = HashMap::new();

    // Spawm thread get_data
    for chain_type in CONFIG.chains.keys() {
        let (chan, _) = broadcast::channel(1024);
        // Clone broadcast channel
        let chan_sender = chan.clone();

        match chain_type {
            // Spawn Substrate get_data
            ChainType::Substrate => {
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::loop_get_block_and_extrinsic(chan_sender).await;
                });
                let chan_sender = chan.clone();
                // Spawn task
                tokio::spawn(async move {
                    substrate_chain::loop_get_event(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Substrate, chan);
            }
            ChainType::Solana => {
                // Spawn task
                tokio::spawn(async move {
                    solana_chain::loop_get_block(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Solana, chan);
            }
            ChainType::Ethereum => {
                // Spawn task
                tokio::spawn(async move {
                    ethereum_chain::loop_get_block(chan_sender).await;
                });
                // add chan to chans
                chans.insert(ChainType::Ethereum, chan);
            }
        }
    }

    // Run StreamoutServer
    let stream_service = StreamService { chans: chans };

    let addr = CONFIG.url.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())
}
