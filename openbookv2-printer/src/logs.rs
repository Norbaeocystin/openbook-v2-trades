use crate::utils::{price_lots_to_ui, to_ui_decimals};
use anchor_lang::prelude::borsh;
use anchor_lang::{event, AnchorDeserialize, AnchorSerialize, Discriminator};
use openbookv2_generated::{FillEvent, Market};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub time_stamp: u64,
    pub maker_owner: String,
    pub taker_owner: String,
    pub price_double: f64,
    pub quantity_double: f64,
    pub market_id: String,
    pub taker_side: u8,
    pub market_name: String,
}

impl Trade {
    pub fn new(fill_log: &FillLog, market: &Market, market_name: String) -> Trade {
        let price_hr = price_lots_to_ui(fill_log.price, market);
        // this is correct
        let quantity = to_ui_decimals(
            fill_log.quantity as f64 * market.base_lot_size as f64,
            market.base_decimals as f64,
        );
        Trade {
            time_stamp: fill_log.timestamp,
            maker_owner: fill_log.maker.to_string(),
            taker_owner: fill_log.taker.to_string(),
            price_double: price_hr,
            quantity_double: quantity,
            market_id: fill_log.market.to_string(),
            taker_side: fill_log.taker_side,
            market_name,
        }
    }
}

#[derive(Debug)]
#[event]
pub struct FillLog {
    pub market: Pubkey,
    pub taker_side: u8, // side from the taker's POV
    pub maker_slot: u8,
    pub maker_out: bool, // true if maker order quantity == 0
    pub timestamp: u64,
    pub seq_num: u64, // note: usize same as u64

    pub maker: Pubkey,
    pub maker_client_order_id: u64,
    pub maker_fee: u64, // native quote

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_client_order_id: u64,
    pub taker_fee_ceil: u64, // native quote

    pub price: i64,
    pub quantity: i64, // number of base lots
}
