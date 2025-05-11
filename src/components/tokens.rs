use serde::{Deserialize, Serialize};
use sled::Db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TokenType {
    FT,
    NFT,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub address: String,
    pub scope: u8,
    pub name: String,
    pub symbol: String,
    pub token_type: TokenType,
    pub decimals: u8,
    pub listed: bool,
    pub status: u8,
}

#[derive(Debug)]
pub struct Tokens {
    db: Db,
}
