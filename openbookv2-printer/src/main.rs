use futures::TryStreamExt;
use std::borrow::BorrowMut;
use std::collections::{BTreeMap, HashSet};
use std::hash::Hasher;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use anchor_lang::__private::base64;
use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator};
use clap::Parser;
// use crossbeam_channel::{Receiver, Sender, unbounded};
use crate::constants::OPENBOOK_V2;
use crate::logs::{FillLog, Trade};
use crate::name::parse_name;
use crate::utils::{get_owner_account_for_ooa, price_lots_to_ui, to_native, to_ui_decimals};
use futures::StreamExt;
use log::{debug, error, info, warn, LevelFilter};
use openbookv2_generated::state::Market;
use openbookv2_generated::FillEvent;
use openbookv2_generated::{id, AnyEvent, EventHeap};
use serde::Serialize;
use serde_json;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{
    RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionLogsConfig,
    RpcTransactionLogsFilter,
};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_client::rpc_response::{Response, RpcLogsResponse};
use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::spawn;
use tokio::sync::mpsc::{channel, unbounded_channel};
use zmq;

pub mod constants;
mod logs;
mod market;
mod name;
mod utils;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
    #[arg(short,long, value_delimiter = ' ', num_args = 1..50, default_value = "AFgkED1FUVfBe2trPUDqSqK9QKd4stJrfzq5q1RwAFTa")]
    // SOL-USDC market default
    market: Vec<String>,
    #[arg(short, long, action)]
    debug: bool,
    #[arg(short, long, default_value = "8585")]
    port: String,
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

// CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 DBSZ24hqXS5o8djunrTzBsJUb1P8ZvBs1nng5rmZKsJt 5h4DTiBqZctQWq7xc3H2t8qRdGcFNQNk1DstVNnbJvXs
fn main() {
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
    let accounts = client.get_multiple_accounts(&market_keys).unwrap();
    let mut event_heap_keys = vec![];
    let mut market_names = BTreeMap::new();
    let mut markets = BTreeMap::new();
    for (idx, option) in accounts.iter().enumerate() {
        let data = option.clone().unwrap().data;
        let market = Market::deserialize(&mut &data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        event_heap_keys.push(market.event_heap);
        market_names.insert(market_keys[idx].clone(), market_name.clone());
        markets.insert(market_keys[idx].clone(), market);
        info!("Polling fills for market: {}", market_name);
    }
    // let mut unsubscribes = vec![];
    // while let Some(response) = subscription.next().await {
    //     if let Some(error) = response.value.err.as_ref() {
    //         // warn!("Skipping TX {:?} with error: {error:?}", response.value.signature);
    //         continue;
    //     }
    //     for log in &response.value.logs {
    //         if log.contains("Program data: ") {
    //             let data = log.replace("Program data: ", "");
    //             let data = base64::decode(data).unwrap();
    //             if discriminator == data.as_slice()[..8] {
    //                 let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
    //                 tx_sender.send((fill_log, response.value.signature.clone())).unwrap();
    //             }
    //         }
    //     }

    let mut ctx = zmq::Context::new();
    let zero_url = format!("tcp://{}:{}", cli.host, cli.port);
    let socket = ctx.socket(zmq::PUB).unwrap();
    socket.bind(&zero_url).unwrap();

    // let mut ooa2owner = BTreeMap::new();
    let mut done = HashSet::new();
    while true {
        let event_heap_accounts: Vec<Account> = client
            .get_multiple_accounts(&event_heap_keys)
            .unwrap()
            .into_iter()
            .map(|acc| acc.unwrap())
            .collect();
        let event_heaps: Vec<EventHeap> = event_heap_accounts
            .into_iter()
            .map(|acc| EventHeap::deserialize(acc.data.as_slice().borrow_mut()).unwrap())
            .collect();
        for event in event_heaps.iter() {
            for node in event.nodes.iter() {
                if node.event.event_type == 0 {
                    let bytes = node.event.try_to_vec().unwrap();
                    let fill_event = FillEvent::deserialize(bytes.as_slice().borrow_mut()).unwrap();
                    fill_event.try_to_vec().unwrap();

                    if fill_event.timestamp != 0 {
                        let hash = hash(&bytes);
                        if !done.contains(&hash) {
                            info!("{:?}", fill_event);
                            done.insert(hash);
                        }
                    }
                }
            }
        }
        sleep(Duration::from_secs(10))
    }
    // while true {
    //     let result = client.get_multiple_accounts(&event_heap_keys);
    //     match result {
    //         Ok(response) => {
    //             let latest_heap_accounts = response
    //                 .into_iter()
    //                 .map(|acc| acc.unwrap())
    //                 .collect();
    //
    //         }
    //         Err(err) => {
    //             warn!("got error: {}", err);
    //         }
    //     }
    // }
    // while let Some((mut fill_log, tx_hash)) = tx_receiver.recv().await {
    //     if let Some(market) = markets.get(&fill_log.market) {
    //         let market_name: &String = market_names.get(&fill_log.market).unwrap();
    //         let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.maker).await;
    //         if result.is_some() {
    //             let maker_owner = result.unwrap();
    //             if ooa2owner.contains_key(&fill_log.maker) {
    //                 ooa2owner.insert(fill_log.maker.clone(), maker_owner.clone());
    //             }
    //             fill_log.maker = maker_owner;
    //         }
    //         let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.taker).await;
    //         if result.is_some() {
    //             let maker_owner = result.unwrap();
    //             if ooa2owner.contains_key(&fill_log.taker) {
    //                 ooa2owner.insert(fill_log.taker.clone(), maker_owner.clone());
    //             }
    //             fill_log.taker = maker_owner;
    //         }
    //         let trade = Trade::new(&fill_log, &market, market_name.clone().replace("\0", ""));
    //         let t = serde_json::to_string(&trade).unwrap();
    //         socket.send(&t, 0);
    //         info!("{:?}, signature: {}", t, tx_hash);
    //     } else {
    //         warn!("tx: {} contains log, which can't be parsed", tx_hash);
    //     }
    // }
}
