use chain_filter::command;
use logger::core::init_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let res = init_logger(&String::from("chain-reader"));
    println!("Log output: {}", res); // Print log output type

    command::run().await
}
