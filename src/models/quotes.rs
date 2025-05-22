use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)] // Added PartialEq, Clone
pub struct TokenQuote {
    pub token_address: String,
    pub eth_price: f64,
    pub timestamp: u64,
}
