use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use thiserror::Error;

use crate::config::rates::BASE_CURRENCY;

#[derive(Error, Debug)]
pub enum MetalsApiError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Invalid API key: {0}")]
    InvalidApiKey(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Missing environment variable: {0}")]
    EnvVar(String),
}

pub async fn get_metals_prices() -> Result<HashMap<String, f64>, MetalsApiError> {
    let api_key = env::var("METALS_API_KEY")
        .map_err(|_| MetalsApiError::EnvVar("METALS_API_KEY not set".to_string()))?;
    let url = format!(
        "https://api.metals.dev/v1/latest?api_key={}&currency={}&unit=g",
        api_key, BASE_CURRENCY,
    );
    let client = Client::new();
    let response = client.get(&url).send().await?;
    let body: Value = response.json().await?;
    parse_metals_response(body)
}

fn parse_metals_response(body: Value) -> Result<HashMap<String, f64>, MetalsApiError> {
    if body["status"].as_str() == Some("failure") {
        let error_message = body["error_message"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        return Err(if body["error_code"].as_i64() == Some(1101) {
            MetalsApiError::InvalidApiKey(error_message)
        } else {
            MetalsApiError::ApiError(error_message)
        });
    }

    let mut result = HashMap::new();

    let metal_tickers = [
        ("aluminum", "XAL"),
        ("copper", "XCU"),
        ("gold", "XAU"),
        ("lead", "XPB"),
        ("nickel", "XNI"),
        ("palladium", "XPD"),
        ("platinum", "XPT"),
        ("silver", "XAG"),
        ("zinc", "XZN"),
    ];

    if let Some(metals) = body["metals"].as_object() {
        for (key, value) in metals {
            if let Some(ticker) = metal_tickers
                .iter()
                .find(|&&(k, _)| k == key)
                .map(|&(_, t)| t)
            {
                if let Some(price) = value.as_f64() {
                    result.insert(ticker.to_string(), price);
                }
            }
        }
    }

    if let Some(currencies) = body["currencies"].as_object() {
        for (key, value) in currencies {
            if let Some(rate) = value.as_f64() {
                result.insert(key.to_string(), rate);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_parse_metals_response_success() {
        let mock = json!({
            "currencies": {
                "AED": 1.0,
                "AFN": 0.05211012,
                "SZL": 0.20355583,
                "XCD": 1.35894319,
                "XDR": 4.95522258,
                "XOF": 0.00624893,
                "XPF": 0.03434988,
                "YER": 0.01503309,
                "ZAR": 0.20355583,
                "ZMW": 0.13807739,
                "ZWD": 0.01014782,
                "ZWL": 0.00005463
            },
            "currency": "AED",
            "metals": {
                "aluminum": 0.009,
                "copper": 0.0373,
                "gold": 378.2465,
                "lead": 0.0073,
                "nickel": 0.0571,
                "palladium": 113.3537,
                "platinum": 116.7301,
                "silver": 3.8126,
                "zinc": 0.0099
            },
            "status": "success",
            "timestamps": {
                "currency": "2025-05-17T03:54:19.725Z",
                "metal": "2025-05-17T03:55:03.494Z"
            },
            "unit": "g"
        });

        let res = parse_metals_response(mock).unwrap();

        assert!(res.contains_key("XAU"), "Expected key XAU");
        assert!(res.contains_key("XAG"), "Expected key XAG");
        assert!(res.contains_key("XPT"), "Expected key XPT");
        assert!(res.contains_key("XPD"), "Expected key XPD");
        assert!(res.contains_key("XAL"), "Expected key XAL");
        assert!(res.contains_key("XCU"), "Expected key XCU");
        assert!(res.contains_key("XPB"), "Expected key XPB");
        assert!(res.contains_key("XNI"), "Expected key XNI");
        assert!(res.contains_key("XZN"), "Expected key XZN");

        assert!(res.contains_key("AED"), "Expected key AED");
        assert!(!res.contains_key("USD"), "USD key should not be present");
        assert!(res.contains_key("ZAR"), "Expected key ZAR");

        assert_eq!(res.get("XAU"), Some(&378.2465), "Incorrect value for XAU");
        assert_eq!(res.get("XAG"), Some(&3.8126), "Incorrect value for XAG");
        assert_eq!(res.get("XPT"), Some(&116.7301), "Incorrect value for XPT");
        assert_eq!(res.get("AED"), Some(&1.0), "Incorrect value for AED");
        assert_eq!(res.get("ZAR"), Some(&0.20355583), "Incorrect value for ZAR");

        assert_eq!(
            res.len(),
            21,
            "Expected 21 elements (9 metals + 12 currencies)"
        );

        assert!(
            !res.contains_key("LBMA_XAU_AM"),
            "LBMA_XAU_AM should not be present"
        );
        assert!(
            !res.contains_key("MCX_XAU"),
            "MCX_XAU should not be present"
        );
    }

    #[tokio::test]
    async fn test_parse_metals_response_invalid_api_key() {
        let mock = json!({
            "error_code": 1101,
            "error_message": "Unauthorized. The API Key provided is invalid.",
            "status": "failure"
        });

        let res = parse_metals_response(mock);
        assert!(res.is_err());
        if let Err(MetalsApiError::InvalidApiKey(msg)) = res {
            assert_eq!(msg, "Unauthorized. The API Key provided is invalid.");
        } else {
            panic!("Expected InvalidApiKey error");
        }
    }

    #[tokio::test]
    async fn test_parse_metals_response_generic_api_error() {
        let mock = json!({
            "error_code": 9999,
            "error_message": "Unknown API error",
            "status": "failure"
        });

        let res = parse_metals_response(mock);
        assert!(res.is_err());
        if let Err(MetalsApiError::ApiError(msg)) = res {
            assert_eq!(msg, "Unknown API error");
        } else {
            panic!("Expected ApiError error");
        }
    }
}
