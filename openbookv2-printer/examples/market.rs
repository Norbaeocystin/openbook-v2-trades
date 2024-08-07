use anchor_lang::{AnchorDeserialize, Discriminator};
use clap::Parser;
use openbookv2_generated::{id, Market};
use serde_json;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use std::str::from_utf8;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

pub fn parse_name(name: &[u8; 16]) -> String {
    let result_name = from_utf8(name);
    // utf8
    if result_name.is_ok() {
        return result_name.unwrap().to_string();
    }
    // utf16
    let u16_bytes: Vec<u16> = name
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    let result_name = String::from_utf16(&u16_bytes);
    if result_name.is_ok() {
        return result_name.unwrap();
    }
    // utf8 with errors
    return String::from_utf8_lossy(name).parse().unwrap();
}

fn main() {
    let cli = Cli::parse();
    let market_filter =
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, Market::discriminator().to_vec()));
    let client = RpcClient::new(&cli.rpc_url);
    let result = client
        .get_program_accounts_with_config(
            &id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![market_filter]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: None,
                    min_context_slot: None,
                },
                with_context: None,
            },
        )
        .unwrap();
    for (key, account) in result.iter() {
        // if key == &Pubkey::from_str("2ekKD6GQy9CPqyqZyFdERr14JcjD5QcJj7DbFfW23k4W").unwrap() {
        let market = Market::deserialize(&mut &account.data[8..]).unwrap();
        let name = parse_name(&market.name);
        println!("market name: {} market id: {}", name, key);
    }
}
