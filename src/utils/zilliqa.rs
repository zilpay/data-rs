use crate::config::zilliqa::{RPCMethod, PROVIDERS, RPC_METHODS};
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::io;

#[derive(Serialize)]
pub struct JsonBody {
    pub id: u8,
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<String>,
}

pub struct Zilliqa {
    providers: Vec<String>,
}

impl Zilliqa {
    pub fn new() -> Self {
        let providers: Vec<String> = PROVIDERS.into_iter().map(|v| String::from(v)).collect();

        Zilliqa { providers }
    }

    pub fn from(providers: Vec<String>) -> Self {
        Zilliqa { providers }
    }

    pub fn extend_providers(&mut self, urls: Vec<String>) {
        self.providers.extend(urls);
    }

    pub async fn fetch(&self, bodies: Vec<JsonBody>) -> Result<Map<String, Value>, io::Error> {
        let client = Client::new();
        let custom_error = std::io::Error::new(std::io::ErrorKind::Other, "Providers is down");
        let content_type = HeaderValue::from_static("application/json");
        let json_str = serde_json::to_string(&bodies)?;

        for provider in self.providers.iter() {
            let request_builder = client.post(provider);
            let request_builder = request_builder.header("Content-Type", &content_type);
            let request_builder = request_builder.json(&json_str);

            let response = match request_builder.send().await {
                Ok(res) => res,
                Err(_) => continue,
            };

            match response.json().await {
                Ok(b) => return Ok(b),
                Err(_) => continue,
            };
        }

        Err(custom_error)
    }

    pub fn build_body(&self, method: &str, params: Vec<String>) -> JsonBody {
        let id = 1;
        let jsonrpc = String::from("2.0");

        JsonBody {
            params,
            id,
            jsonrpc,
            method: String::from(method),
        }
    }
}
