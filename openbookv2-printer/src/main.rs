use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use futures::TryStreamExt;
use std::borrow::BorrowMut;

use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator};
use anchor_lang::__private::base64;
use clap::Parser;
// use crossbeam_channel::{Receiver, Sender, unbounded};
use futures::StreamExt;
use log::{debug, error, info, LevelFilter};
use serde::Serialize;
use serde_json;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_client::rpc_response::{Response, RpcLogsResponse};
use solana_program::pubkey::Pubkey;
use tokio::spawn;
use tokio::sync::mpsc::{channel, unbounded_channel};
use zmq;
use openbookv2_generated::id;
use openbookv2_generated::state::Market;
use openbookv2_generated::FillEvent;
use crate::constants::OPENBOOK_V2;
use crate::logs::{FillLog, Trade};
use crate::name::parse_name;
use crate::utils::{get_owner_account_for_ooa, price_lots_to_ui, to_native, to_ui_decimals};

pub mod constants;
mod name;
mod market;
mod logs;
mod utils;

#[derive(Parser)]
struct Cli {
    #[arg(short,long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
    #[arg(short,long, value_delimiter = ' ', num_args = 1..50, default_value = "CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3")] // SOL-USDC market default
    market: Vec<String>,
    #[arg(short, long, action)]
    debug: bool,
    #[arg(short,long, default_value = "8585")]
    port: String,
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

// CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 DBSZ24hqXS5o8djunrTzBsJUb1P8ZvBs1nng5rmZKsJt 5h4DTiBqZctQWq7xc3H2t8qRdGcFNQNk1DstVNnbJvXs
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env_logger::builder().filter_level(LevelFilter::Debug).init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Info).init();
    }
    let client = RpcClient::new(cli.rpc_url.clone());
    let market_keys = cli.market.iter().map(|market_key| Pubkey::from_str(market_key).unwrap()).collect::<Vec<Pubkey>>();
    let accounts = client.get_multiple_accounts(&market_keys).await.unwrap();
    let mut event_heaps = vec![];
    for option in accounts {
        let data = option.unwrap().data;
        let market = Market::deserialize(&mut &data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        event_heaps.push(market.event_heap.to_string());
        info!("Listening to fills for market: {}", market_name.clone());
    }
    let wss_url = cli.rpc_url.replace("https://", "wss://");
    // let mut unsubscribes = vec![];
    let pubsub_client: Arc<PubsubClient> = Arc::new(PubsubClient::new(&wss_url).await.unwrap());
    let (tx_sender, mut tx_receiver) = unbounded_channel::<(FillLog, String, usize)>();
    for (idx,event_heap) in event_heaps.iter().enumerate() {
        debug!("subscribing to event heap: {}", event_heap);
        let discriminator = FillLog::discriminator();
        let tx_sender = tx_sender.clone();
        let event_heap = event_heap.clone();
        let clone = pubsub_client.clone();
        spawn( async move {
            let (ref mut subscription, unsubscribe) = clone.logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![event_heap.to_string()]),
                RpcTransactionLogsConfig { commitment: None }
            ).await.unwrap();
            drop(unsubscribe);
            while let Some(response) = subscription.next().await {
                if let Some(error) = response.value.err.as_ref() {
                    // warn!("Skipping TX {:?} with error: {error:?}", response.value.signature);
                    continue;
                }
                for log in &response.value.logs {
                    if log.contains("Program data: ") {
                        let data = log.replace("Program data: ", "");
                        let data = base64::decode(data).unwrap();
                        if discriminator == data.as_slice()[..8] {
                            let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
                            tx_sender.send((fill_log, response.value.signature.clone(), idx)).unwrap();
                        }
                    }
                }
            }
        });
    }

    let mut ctx = zmq::Context::new();
    let zero_url = format!("tcp://{}:{}", cli.host, cli.port);
    let socket = ctx.socket(zmq::PUB).unwrap();
    socket.bind(&zero_url).unwrap();

    let mut ooa2owner = BTreeMap::new();
    let accounts = client.get_multiple_accounts(&market_keys).await.unwrap();
    let mut market_names = vec![];
    let mut markets = vec![];
    for option in accounts {
        let data = option.unwrap().data;
        let market = Market::deserialize(&mut &data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        market_names.push(market_name);
        markets.push(market);
    }
    while let Some((mut fill_log, tx_hash, idx)) = tx_receiver.recv().await {
        let market = markets.get(idx).unwrap();
        let market_name: &String = market_names.get(idx).unwrap();
        let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.maker).await;
        if result.is_some() {
            let maker_owner = result.unwrap();
            if ooa2owner.contains_key(&fill_log.maker) {
                ooa2owner.insert(fill_log.maker.clone(), maker_owner.clone());
            }
            fill_log.maker = maker_owner;
        }
        let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.taker).await;
        if result.is_some() {
            let maker_owner = result.unwrap();
            if ooa2owner.contains_key(&fill_log.taker) {
                ooa2owner.insert(fill_log.taker.clone(), maker_owner.clone());
            }
            fill_log.taker = maker_owner;
        }
        let trade = Trade::new(&fill_log, &market, market_name.clone().replace("\0", ""));
        let t = serde_json::to_string(&trade).unwrap();
        socket.send(&t, 0);
        info!("{:?}, signature: {}", t, tx_hash);
    }
}
