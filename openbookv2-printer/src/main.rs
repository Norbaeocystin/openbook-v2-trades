use anchor_lang::__private::base64;
use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator};
use clap::Parser;
use futures::TryStreamExt;
use std::borrow::BorrowMut;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hasher;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
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
use solana_client::nonblocking::rpc_client::RpcClient;
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
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::CommitmentLevel;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::{
    SubscribeRequest, SubscribeRequestFilterAccountsFilterMemcmp,
    SubscribeRequestFilterTransactions, SubscribeUpdate, SubscribeUpdateAccount,
};
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
    #[arg(short, long, default_value = "http://127.0.0.1:10000")]
    grpc: String,
    #[clap(value_enum, default_value = "finalized")]
    commitment: Commitment,
}

#[derive(clap::ValueEnum, Clone)]
enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

// CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 DBSZ24hqXS5o8djunrTzBsJUb1P8ZvBs1nng5rmZKsJt 5h4DTiBqZctQWq7xc3H2t8qRdGcFNQNk1DstVNnbJvXs
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
        market_names.insert(market_keys[idx].clone(), market_name.clone());
        markets.insert(market_keys[idx].clone(), market);
        info!("Polling fills for market: {}", market_name);
    }

    let mut grpc_client = GeyserGrpcClient::build_from_shared(cli.grpc)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let pong = grpc_client.ping(0).await.unwrap();
    info!("{:?}", pong);

    let mut transactions = HashMap::new();
    for key in market_keys.iter() {
        let tx_filter = SubscribeRequestFilterTransactions {
            vote: None,
            failed: None,
            signature: None,
            account_include: vec![],
            account_exclude: vec![],
            account_required: vec![key.to_string()],
        };
        transactions.insert(key.to_string(), tx_filter);
    }
    let commitment = match cli.commitment {
        Commitment::Processed => {
            CommitmentLevel::Processed
        }
        Commitment::Confirmed => {
            CommitmentLevel::Confirmed
        }
        Commitment::Finalized => {
            CommitmentLevel::Finalized
        }
    };
    let request = SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions: transactions,
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: Some(i32::from(commitment)),
        accounts_data_slice: vec![],
        ping: None,
        transactions_status: Default::default(),
    };
    let (_subscribe_tx, mut stream) = grpc_client
        .subscribe_with_request(Some(request))
        .await
        .unwrap();
    while let Some(message) = stream.next().await {
        if let Ok(msg) = message {
            debug!("new message: {msg:?}");
            let market = msg.filters.first().unwrap();
            #[allow(clippy::single_match)]
            match msg.update_oneof {
                Some(UpdateOneof::Transaction(tx)) => {
                    let tx = tx.transaction.unwrap();
                    let logs = tx.meta.unwrap().log_messages;
                    // TODO process logs
                }
                _ => {}
            }
        }
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
}
