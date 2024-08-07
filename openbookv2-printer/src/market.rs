use anchor_lang::{AnchorDeserialize, Discriminator};
use openbookv2_generated::{id, Market};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::Memcmp;
use solana_client::rpc_filter::RpcFilterType;
use solana_program::pubkey::Pubkey;

pub fn get_all_markets(client: RpcClient) -> Vec<(Pubkey, Market)> {
    let market_filter =
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, Market::discriminator().to_vec()));
    let result = client
        .get_program_accounts_with_config(
            &id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![market_filter]),
                account_config: RpcAccountInfoConfig {
                    encoding: None,
                    data_slice: None,
                    commitment: None,
                    min_context_slot: None,
                },
                with_context: None,
            },
        )
        .unwrap();
    let mut key_and_market = vec![];
    for (key, account) in result.iter() {
        // if key == &Pubkey::from_str("2ekKD6GQy9CPqyqZyFdERr14JcjD5QcJj7DbFfW23k4W").unwrap() {
        let market = Market::deserialize(&mut &account.data[8..]).unwrap();
        // let name = parse_name(&market.name);
        key_and_market.push((*key, market))
    }
    key_and_market
}
