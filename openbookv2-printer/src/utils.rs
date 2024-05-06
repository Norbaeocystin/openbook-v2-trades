use std::collections::BTreeMap;
use anchor_lang::Discriminator;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use openbookv2_generated::{Market, OpenOrdersAccount};

pub fn to_native(value: f64, decimals: f64) -> f64 {
    let d = 10_f64.powf(decimals);
    return return value * d;
}

pub fn to_ui_decimals(value: f64, decimals: f64) -> f64 {
    let d = 10_f64.powf(decimals);
    return return value/d;
}

pub fn price_lots_to_ui(price: i64, market: &Market) -> f64 {
    let coeff = (10_f64.powf((market.base_decimals as i8 - market.quote_decimals as i8) as f64) * market.quote_lot_size as f64)/market.base_lot_size as f64;
    return price as f64 * coeff
}

pub async fn get_owner_account_for_ooa(client: &RpcClient, ooa2owner: &BTreeMap<Pubkey, Pubkey> , key: &Pubkey) -> Option<Pubkey> {
    if !ooa2owner.contains_key(key) {
        let mut data = client.get_account_data(key).await.unwrap();
        if data.len() > 8 {
            if data[0..8] == OpenOrdersAccount::discriminator() {
                let pubkey_data: [u8; 32] = data.drain(8..40).collect::<Vec<u8>>().try_into().unwrap();
                return Some(Pubkey::from(pubkey_data));
            } else {
                return None;
            }
        }
    } else {
        return Some(ooa2owner.get(key).unwrap().clone());
    }
    return None;
}