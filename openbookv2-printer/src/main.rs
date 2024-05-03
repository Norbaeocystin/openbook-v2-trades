use std::collections::BTreeMap;
use std::os::unix::raw::off_t;
use std::str::FromStr;
use std::thread::spawn;

use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator};
use anchor_lang::__private::base64;
use clap::Parser;
use crossbeam_channel::{Receiver, Sender, unbounded};
use log::{debug, error, info, LevelFilter};
use serde::Serialize;
use serde_json;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_program::pubkey::Pubkey;
use openbookv2_generated::id;
use openbookv2_generated::state::Market;
use openbookv2_generated::FillEvent;
use zmq;
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
    #[arg(short,long, default_value = "CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3")] // SOL-USDC market default
    market: String,
    #[arg(short, long, action)]
    debug: bool,
    #[arg(short,long, default_value = "5555")]
    port: String,
    #[arg(short,long, default_value = "127.0.0.1")]
    host: String,
}

fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env_logger::builder().filter_level(LevelFilter::Debug).init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Info).init();
    }
    let client = RpcClient::new(&cli.rpc_url);
    let market_data = client.get_account_data(&Pubkey::from_str(&cli.market).unwrap()).unwrap();
    let market = Market::deserialize(&mut &market_data[8..]).unwrap();
    let market_name = parse_name(&market.name);
    debug!("Market: {}", market_name.clone());
    let wss_url = cli.rpc_url.replace("https://", "wss://");
    let (subscription, receiver) = PubsubClient::logs_subscribe(&wss_url,
       RpcTransactionLogsFilter::Mentions(vec![market.event_heap.to_string()]),
        RpcTransactionLogsConfig{ commitment: None }
    ).unwrap();
    let discriminator = FillLog::discriminator();
    let (tx_sender, tx_receiver):(Sender<(FillLog,String)>, Receiver<(FillLog,String)>) = unbounded();
    spawn(move || {
        let mut ctx = zmq::Context::new();
        let zero_url = format!("tcp://{}:{}", cli.host, cli.port);
        let socket = ctx.socket(zmq::PUB).unwrap();
        socket.bind(&zero_url).unwrap();
        let mut ooa2owner = BTreeMap::new();
        let market_data = client.get_account_data(&Pubkey::from_str(&cli.market).unwrap()).unwrap();
        let market = Market::deserialize(&mut &market_data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        info!("Market: {}", market_name.clone());
        loop {
            let result = tx_receiver.recv();
            if result.is_ok() {
                let (mut fill_log, tx_hash) = result.unwrap();
                // fetch owner of ooa and store it in the
                let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.maker);
                if result.is_some() {
                    let maker_owner = result.unwrap();
                    if ooa2owner.contains_key(&fill_log.maker) {
                        ooa2owner.insert(fill_log.maker.clone(), maker_owner.clone());
                    }
                    fill_log.maker = maker_owner;
                }
                let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.taker);
                if result.is_some() {
                    let maker_owner = result.unwrap();
                    if ooa2owner.contains_key(&fill_log.taker) {
                        ooa2owner.insert(fill_log.taker.clone(), maker_owner.clone());
                    }
                    fill_log.taker = maker_owner;
                }
                let trade = Trade::new(&fill_log, &market, market_name.clone().replace("\0",""));
                let t = serde_json::to_string(&trade).unwrap();
                socket.send(&t, 0);
                info!("{:?}, signature: {}", t, tx_hash);
            }
        }
    });
    loop {
        match receiver.recv() {
            Ok(response) => {
                // remove logs if contains
                let any = response.value.logs.iter().any(|x|x.contains("error"));
                if any {
                    continue;
                }
                for log in &response.value.logs{
                    if log.contains("Program data: ") {
                        let data = log.replace("Program data: ", "");
                        let data = base64::decode(data).unwrap();
                        if discriminator == data.as_slice()[..8] {
                            let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
                            tx_sender.send((fill_log, response.value.signature.clone())).unwrap()
                            // let trade = Trade::new(&fill_log, &market, market_name.clone().replace("\0",""));
                            // let t = serde_json::to_string(&trade).unwrap();
                            // info!("{:?}, signature: {}", t, response.value.signature);
                        }
                    }
                    // println!("{}", log);
                }
            }
            Err(e) => {
                error!("account subscription error: {:?}", e);
                break;
            }
        }
    }
}
