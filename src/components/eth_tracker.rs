use super::{
    tokens::{coingecko_get_tokens, Token},
    uniswap::get_token_prices_in_eth,
};
use bincode::config::{self, Configuration};
use sled;
use std::error::Error;

const ETH_STORAGE_KEY: &[u8] = b"eth_tokens";

pub struct EthTracker {
    db: sled::Db,
    bincode_config: Configuration,
}

impl EthTracker {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let db = sled::open("data-rs/eth_tracker_db")?;
        let bincode_config = config::standard();

        Ok(EthTracker { db, bincode_config })
    }

    pub fn get_tokens(&self) -> Result<Vec<Token>, Box<dyn Error>> {
        if let Some(encoded) = self.db.get(ETH_STORAGE_KEY)? {
            let (tokens, _): (Vec<Token>, usize) =
                bincode::decode_from_slice(&encoded[..], self.bincode_config).unwrap();

            Ok(tokens)
        } else {
            Ok(vec![])
        }
    }

    pub fn save_tokens(&self, tokens: Vec<Token>) -> Result<(), Box<dyn Error>> {
        let bytes = bincode::encode_to_vec(tokens, self.bincode_config)?;

        self.db.insert(ETH_STORAGE_KEY, bytes)?;
        self.db.flush()?;

        Ok(())
    }

    pub async fn update_tokens_from_coingecko(&self) -> Result<(), Box<dyn Error>> {
        let mut current_tokens = self.get_tokens()?;
        let new_tokens = coingecko_get_tokens("ethereum").await?;
        for new_token in new_tokens {
            if !current_tokens
                .iter()
                .any(|t| t.address == new_token.address)
            {
                current_tokens.push(new_token);
            }
        }
        self.save_tokens(current_tokens)?;
        Ok(())
    }

    pub async fn update_rates_from_uniswap(&self) -> Result<(), Box<dyn Error>> {
        let mut tokens = self.get_tokens()?;
        get_token_prices_in_eth(&mut tokens).await?;
        self.save_tokens(tokens)?;
        Ok(())
    }
}
