use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use futures::StreamExt;
use log::{debug, error, info, warn, LevelFilter};
use clap::Parser;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anchor_lang::__private::base64;
use serde_json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::spawn;
use tokio::sync::mpsc::unbounded_channel;
use zmq;
use crate::logs::{FillLog, Trade};
use crate::name::parse_name;
use crate::utils::get_owner_account_for_ooa;
use openbookv2_generated::state::Market;

pub mod constants;
mod logs;
mod market;
mod name;
mod utils;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
    #[arg(short, long, value_delimiter = ' ', num_args = 1..50, default_value = "CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3")]
    market: Vec<String>,
    #[arg(short, long, action)]
    debug: bool,
    #[arg(short, long, default_value = "8585")]
    port: String,
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(short, long, action)]
    connect: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Info).init();
    }

    let processed_commitment = CommitmentConfig::processed();
    let client = RpcClient::new_with_commitment(cli.rpc_url.clone(), processed_commitment);
    let market_keys = cli
        .market
        .iter()
        .map(|market_key| Pubkey::from_str(market_key).unwrap())
        .collect::<Vec<Pubkey>>();
    let accounts = client.get_multiple_accounts(&market_keys).await.unwrap();

    let mut event_heaps = vec![];
    let mut market_names = BTreeMap::new();
    let mut markets = BTreeMap::new();

    for (idx, option) in accounts.iter().enumerate() {
        let data = option.clone().unwrap().data;
        let market = Market::deserialize(&mut &data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        event_heaps.push(market.event_heap.to_string());
        market_names.insert(market_keys[idx], market_name.clone());
        markets.insert(market_keys[idx], market);
        info!("Listening to fills for market: {}", market_name);
    }

    let wss_url = cli.rpc_url.replace("https://", "wss://");
    let pubsub_client: Arc<PubsubClient> = Arc::new(PubsubClient::new(&wss_url).await.unwrap());
    let (tx_sender, mut tx_receiver) = unbounded_channel::<(FillLog, String)>();
    let discriminator = FillLog::discriminator();

    for event_heap in event_heaps.iter() {
        debug!("Subscribing to event heap: {}", event_heap);
        let tx_sender = tx_sender.clone();
        let event_heap = event_heap.clone();
        let clone = pubsub_client.clone();

        spawn(async move {
            let (ref mut subscription, unsubscribe) = clone
                .logs_subscribe(
                    RpcTransactionLogsFilter::Mentions(vec![event_heap.to_string()]),
                    RpcTransactionLogsConfig { commitment: None },
                )
                .await
                .unwrap();

            drop(unsubscribe);
            while let Some(response) = subscription.next().await {
                if let Some(error) = response.value.err.as_ref() {
                    warn!("Skipping TX {:?} with error: {:?}", response.value.signature, error);
                    continue;
                }
                for log in &response.value.logs {
                    if log.contains("Program data: ") {
                        let data = log.replace("Program data: ", "");
                        let data = base64::decode(data).unwrap();
                        if discriminator == data.as_slice()[..8] {
                            let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
                            tx_sender.send((fill_log, response.value.signature.clone())).unwrap();
                        }
                    }
                }
            }
        });
    }

    let ctx = zmq::Context::new();
    let zero_url = format!("tcp://{}:{}", cli.host, cli.port);
    let socket = ctx.socket(zmq::PUB).unwrap();
    if cli.connect {
        socket.connect(&zero_url).unwrap()
    } else {
        socket.bind(&zero_url).unwrap();
    }

    let mut ooa2owner = BTreeMap::new();
    while let Some((mut fill_log, tx_hash)) = tx_receiver.recv().await {
        if let Some(market) = markets.get(&fill_log.market) {
            let market_name: &String = market_names.get(&fill_log.market).unwrap();

            if let Some(maker_owner) = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.maker).await {
                ooa2owner.insert(fill_log.maker, maker_owner.clone());
                fill_log.maker = maker_owner;
            }
            if let Some(taker_owner) = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.taker).await {
                ooa2owner.insert(fill_log.taker, taker_owner.clone());
                fill_log.taker = taker_owner;
            }

            let trade = Trade::new(&fill_log, market, market_name.clone().replace('\0', ""));
            let t = serde_json::to_string(&trade).unwrap();
            match socket.send(&t, 0) {
                Ok(_) => {}
                Err(err) => {
                    error!("Sending to socket returned error: {}", err);
                }
            }
            info!("{:?}, signature: {}", t, tx_hash);
        } else {
            warn!("TX: {} contains log, which can't be parsed", tx_hash);
        }
    }
}
