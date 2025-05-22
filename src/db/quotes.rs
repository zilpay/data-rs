use crate::models::quotes::TokenQuote;
use sled::Db;
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuoteDbError {
    #[error("Sled DB error: {0}")]
    SledError(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Environment variable DB_PATH not set")]
    DbPathNotSet,
}

fn open_db() -> Result<Db, QuoteDbError> {
    let db_path = env::var("DB_PATH").map_err(|_| QuoteDbError::DbPathNotSet)?;
    sled::open(db_path).map_err(QuoteDbError::SledError)
}

const QUOTES_TREE_NAME: &str = "token_quotes";

pub fn save_quote(token_address: &str, quote: &TokenQuote) -> Result<(), QuoteDbError> {
    let db = open_db()?;
    let tree = db.open_tree(QUOTES_TREE_NAME)?;
    let serialized_quote = serde_json::to_string(quote)?;
    tree.insert(token_address.as_bytes(), serialized_quote.as_bytes())?;
    db.flush()?;
    Ok(())
}

pub fn get_quote(token_address: &str) -> Result<Option<TokenQuote>, QuoteDbError> {
    let db = open_db()?;
    let tree = db.open_tree(QUOTES_TREE_NAME)?;
    match tree.get(token_address.as_bytes())? {
        Some(ivec) => {
            let serialized_quote = String::from_utf8(ivec.to_vec())
                .map_err(|e| QuoteDbError::SerializationError(serde_json::Error::custom(e.to_string())))?;
            let quote: TokenQuote = serde_json::from_str(&serialized_quote)?;
            Ok(Some(quote))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::env;
    use crate::models::quotes::TokenQuote; // Ensure TokenQuote is in scope for tests

    // Helper to set up DB_PATH for a test
    struct TestDbGuard {
        _temp_dir: tempfile::TempDir, // Keep TempDir in scope to prevent premature deletion
        original_db_path: Option<String>,
    }

    impl TestDbGuard {
        fn new() -> Self {
            let temp_dir = tempdir().unwrap();
            let temp_path = temp_dir.path().to_str().unwrap().to_string();
            
            let original_db_path = env::var("DB_PATH").ok();
            env::set_var("DB_PATH", &temp_path);
            
            TestDbGuard {
                _temp_dir: temp_dir,
                original_db_path,
            }
        }
    }

    impl Drop for TestDbGuard {
        fn drop(&mut self) {
            if let Some(original_path) = &self.original_db_path {
                env::set_var("DB_PATH", original_path);
            } else {
                env::remove_var("DB_PATH");
            }
        }
    }

    #[test]
    fn save_and_get_quote_success() {
        let _guard = TestDbGuard::new(); // Sets up and tears down DB_PATH

        let token_addr = "0xTEST_TOKEN_ADDRESS";
        let sample_quote = TokenQuote {
            token_address: token_addr.to_string(),
            eth_price: 0.0005,
            timestamp: 1234567890,
        };

        // Save the quote
        let save_result = save_quote(token_addr, &sample_quote);
        assert!(save_result.is_ok());

        // Retrieve the quote
        let retrieved_result = get_quote(token_addr);
        assert!(retrieved_result.is_ok());
        
        match retrieved_result.unwrap() {
            Some(retrieved_quote) => {
                // Due to potential f64 precision issues, direct comparison can be tricky.
                // For this test, TokenQuote now derives PartialEq, which should work for these values.
                assert_eq!(sample_quote, retrieved_quote);
            }
            None => panic!("Quote not found after saving"),
        }
    }

    #[test]
    fn get_non_existent_quote() {
        let _guard = TestDbGuard::new(); // Sets up and tears down DB_PATH

        let token_addr = "0xNON_EXISTENT_ADDRESS";
        
        let result = get_quote(token_addr);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn save_quote_updates_existing() {
        let _guard = TestDbGuard::new();

        let token_addr = "0xUPDATE_TEST_TOKEN";
        let initial_quote = TokenQuote {
            token_address: token_addr.to_string(),
            eth_price: 0.001,
            timestamp: 1000,
        };
        let updated_quote = TokenQuote {
            token_address: token_addr.to_string(),
            eth_price: 0.002,
            timestamp: 2000,
        };

        // Save initial quote
        assert!(save_quote(token_addr, &initial_quote).is_ok());
        let retrieved_initial = get_quote(token_addr).unwrap().unwrap();
        assert_eq!(initial_quote, retrieved_initial);

        // Save updated quote for the same address
        assert!(save_quote(token_addr, &updated_quote).is_ok());
        let retrieved_updated = get_quote(token_addr).unwrap().unwrap();
        assert_eq!(updated_quote, retrieved_updated);
    }
}
