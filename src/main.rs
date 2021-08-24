mod miner;

use miner::mine;
use std::env;

#[tokio::main]
async fn main() {
    let mut args = env::args();

    let _ = args.next();
    let region = args.next().unwrap();

    mine(region).await;
}
