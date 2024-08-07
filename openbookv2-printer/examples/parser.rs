use anchor_lang::prelude::borsh;
use anchor_lang::{event, AnchorDeserialize, AnchorSerialize, Discriminator};
use solana_program::pubkey::Pubkey;

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

pub fn main() {
    let data = vec![
        "lhcplJii10CnI1yNu/8vXkUqgSU3o20mSjzZtx/d7eCUeQzZACE/qgEAAEJIMWYAAAAAohcBAAAAAABkvCMD57g3rSwu61+YmqaAUgJhSvu60xT3sb+E4lUQ4wAAAAAAAAAAyAQAAAAAAABCSDFmAAAAACLJm4k4/WcLchsoYrnq3admVRzfbii7s68EbKm0dtXpAAAAAAAAAADJBAAAAAAAAHXeAQAAAAAACgAAAAAAAAA=",
        "xPmUIajkSQYL7zHCrjpsMNohieUs9fA6jy988YbbplZ7EexNmkPC3QEAAACnI1yNu/8vXkUqgSU3o20mSjzZtx/d7eCUeQzZACE/qo/nAAAAAAAA52yzsAEAAACn+QAAAAAAAICWmAAAAAAAAAAAAAAAAAB/xW4AAAAAAMgEAAAAAAAAAPOfhtY+AAAAAAAAAAAAANZP9acAAAAAAAAAAAAAAAA=",
        "COswOq5MnGkBIsmbiTj9ZwtyGyhiuerdp2ZVHN9uKLuzrwRsqbR21emAlpgAAAAAAMmrEgAAAAAAyQQAAAAAAAA=",
        ];
    for item in data.iter() {
        let data = anchor_lang::__private::base64::decode(item).unwrap();
        let discriminator = FillLog::discriminator();
        if discriminator == data.as_slice()[..8] {
            let fill_log = FillLog::deserialize(&mut &data[8..]).unwrap();
            println!("{:?}", fill_log);
        }
    }
}
