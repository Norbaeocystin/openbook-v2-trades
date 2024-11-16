use crate::logs::{FillLog, Trade};
use crate::name::parse_name;
use crate::utils::{get_owner_account_for_ooa, price_lots_to_ui, to_native, to_ui_decimals};
use anchor_lang::__private::base64;
use anchor_lang::{AnchorDeserialize, AnchorSerialize, Discriminator};
use clap::Parser;
use futures::StreamExt;
use log::{debug, error, info, warn, LevelFilter};
use openbookv2_generated::state::Market;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use tokio::spawn;
use tokio::sync::mpsc::unbounded_channel;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::CommitmentLevel;
use yellowstone_grpc_proto::prelude::{SubscribeRequest, SubscribeRequestFilterTransactions};

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
    #[arg(short, long, action)]
    connect: bool,
    #[arg(short, long, default_value = "x-token")]
    x_token: String,
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
    let mut market_names = BTreeMap::new();
    let mut markets = BTreeMap::new();
    for (idx, option) in accounts.iter().enumerate() {
        let data = option.clone().unwrap().data;
        let market = Market::deserialize(&mut &data[8..]).unwrap();
        let market_name = parse_name(&market.name);
        market_names.insert(market_keys[idx], market_name.clone());
        markets.insert(market_keys[idx], market);
        info!("Polling fills for market: {}", market_name);
    }

    let mut grpc_client = GeyserGrpcClient::build_from_shared(cli.grpc)
        .unwrap()
        .x_token(Some(cli.x_token.clone()))
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
            failed: Some(false),
            signature: None,
            account_include: vec![],
            account_exclude: vec![],
            account_required: vec![key.to_string()],
        };
        transactions.insert(key.to_string(), tx_filter);
    }
    let commitment = match cli.commitment {
        Commitment::Processed => CommitmentLevel::Processed,
        Commitment::Confirmed => CommitmentLevel::Confirmed,
        Commitment::Finalized => CommitmentLevel::Finalized,
    };
    let request = SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions,
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: Some(i32::from(commitment)),
        accounts_data_slice: vec![],
        ping: None,
        transactions_status: Default::default(),
    };

    let (tx_sender, mut tx_receiver) = unbounded_channel::<(FillLog, String)>();
    let discriminator = FillLog::discriminator();
    let request = request.clone();
    spawn(async move {
        loop {
            let (_subscribe_tx, mut stream) = grpc_client
                .subscribe_with_request(Some(request.clone()))
                .await
                .unwrap();
            while let Some(message) = stream.next().await {
              if let Ok(msg) =  message {
                        debug!("new message: {msg:?}");
                        # [allow(clippy::single_match)]
                            match msg.update_oneof {
                                Some(UpdateOneof::Transaction(tx)) => {
                                    let tx = tx.transaction.unwrap();
                                    let logs = tx.meta.unwrap().log_messages;
                                    for log in logs.iter() {
                                        if log.contains("Program data: ") {
                                            let data = log.replace("Program data: ", "");
                                            let data = base64::decode(data).unwrap();
                                            if discriminator == data.as_slice()[..8] {
                                                let signature = Signature::new(&tx.signature).to_string();
                                                let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
                                                tx_sender.send((fill_log, signature)).unwrap();
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                    }
                }
        }
    });

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
            let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.maker).await;
            if result.is_some() {
                let maker_owner = result.unwrap();
                if ooa2owner.contains_key(&fill_log.maker) {
                    ooa2owner.insert(fill_log.maker, maker_owner);
                }
                fill_log.maker = maker_owner;
            }
            let result = get_owner_account_for_ooa(&client, &ooa2owner, &fill_log.taker).await;
            if result.is_some() {
                let maker_owner = result.unwrap();
                if ooa2owner.contains_key(&fill_log.taker) {
                    ooa2owner.insert(fill_log.taker, maker_owner);
                }
                fill_log.taker = maker_owner;
            }
            let trade = Trade::new(&fill_log, market, market_name.clone().replace('\0', ""));
            let t = serde_json::to_string(&trade).unwrap();
            let r = socket.send(&t, 0);
            match r {
                Ok(_) => {}
                Err(err) => {
                    error!("sending to socket returned error: {}", err);
                }
            }
            info!("{:?}, signature: {}", t, tx_hash);
        } else {
            warn!("tx: {} contains log, which can't be parsed, because does not contain specified market", tx_hash);
        }
    }
}
