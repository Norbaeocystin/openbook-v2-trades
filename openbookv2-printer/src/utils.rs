use openbookv2_generated::Market;

pub fn to_native(value: f64, decimals: f64) -> f64 {
    let d = 10_f64.powf(decimals);
    return return value * d;
}

pub fn to_ui_decimals(value: f64, decimals: f64) -> f64 {
    let d = 10_f64.powf(decimals);
    return return value/d;
}

pub fn price_lots_to_ui(price: i64, market: &Market) -> f64 {
    let coeff = (10_f64.powf((market.base_decimals - market.quote_decimals) as f64) * market.quote_lot_size as f64)/market.base_lot_size as f64;
    return price as f64 * coeff
}