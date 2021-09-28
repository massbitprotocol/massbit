use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;

use crate::ethereum_chain;
use chain_ethereum::network::{EthereumNetworkAdapter, EthereumNetworkAdapters};
use chain_ethereum::{manifest, Chain, EthereumAdapter, Transport};
use log::{error, info};
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::sync::Arc;
use stream_mod::{
    streamout_server::Streamout, ChainType, GenericDataProto, GetBlocksRequest, HelloReply,
    HelloRequest,
};
use tonic::{Request, Response, Status};

const QUEUE_BUFFER: usize = 1024;

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

#[derive(Debug)]
pub struct StreamService {
    pub chans: HashMap<(ChainType, NetworkType), broadcast::Sender<GenericDataProto>>,
    pub chains: HashMap<(ChainType, NetworkType), Arc<Chain>>,
}

async fn create_ethereum_adapter() -> EthereumAdapter {
    let (transport_event_loop, transport) =
        Transport::new_rpc("https://rpc-mainnet.matic.network", Default::default());

    // If we drop the event loop the transport will stop working.
    // For now it's fine to just leak it.
    std::mem::forget(transport_event_loop);

    chain_ethereum::EthereumAdapter::new(
        "matic".to_string(),
        "https://rpc-mainnet.matic.network",
        transport,
        false,
    )
    .await
}

#[tonic::async_trait]
impl Streamout for StreamService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Got a request: {:?}", request);

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }

    type ListBlocksStream = ReceiverStream<Result<GenericDataProto, Status>>;

    async fn list_blocks(
        &self,
        request: Request<GetBlocksRequest>,
    ) -> Result<Response<Self::ListBlocksStream>, Status> {
        info!("Request = {:?}", request);
        let chain_type: ChainType = ChainType::from_i32(request.get_ref().chain_type).unwrap();
        let network: NetworkType = request.get_ref().network.clone();
        let start_block = request.get_ref().start_block_number;
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);
        match chain_type {
            ChainType::Substrate | ChainType::Solana => {
                // tx, rx for out stream gRPC
                // let (tx, rx) = mpsc::channel(1024);

                // Create new channel for connect between input and output stream
                println!(
                    "chains: {:?}, chain_type: {:?}, network: {}",
                    &self.chans, chain_type, network
                );

                let mut rx_chan = self.chans.get(&(chain_type, network)).unwrap().subscribe();

                tokio::spawn(async move {
                    loop {
                        // Getting generic_data
                        let generic_data = rx_chan.recv().await.unwrap();
                        // Send generic_data to queue"
                        let res = tx.send(Ok(generic_data)).await;
                        if res.is_err() {
                            error!("Cannot send data to RPC client queue, error: {:?}", res);
                        }
                    }
                });
            }
            ChainType::Ethereum => {
                let name = "deployment".to_string();
                let chain = self
                    .chains
                    .get(&(chain_type, network.clone()))
                    .unwrap()
                    .clone();

                graph::spawn_thread(name, move || {
                    graph::block_on(task::unconstrained(async {
                        let start_block = match start_block {
                            0 => None,
                            _ => Some(start_block),
                        };

                        let resp = ethereum_chain::loop_get_block(
                            tx.clone(),
                            &start_block,
                            &network,
                            chain,
                        )
                        .await;

                        error!("Stop loop_get_block, error: {:?}", resp);
                    }))
                });
            }
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
