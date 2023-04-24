use crate::config::zilliqa::PROVIDERS;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Debug)]
pub struct JsonBodyReq {
    pub id: String,
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct JsonBodyRes<T> {
    pub result: Option<T>,
}

#[derive(Debug)]
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

    pub async fn fetch<T: DeserializeOwned>(
        &self,
        bodies: Vec<JsonBodyReq>,
    ) -> Result<Vec<JsonBodyRes<T>>, io::Error> {
        let client = Client::new();
        let custom_error = std::io::Error::new(std::io::ErrorKind::Other, "Providers is down");

        for provider in self.providers.iter() {
            let mut headers = HeaderMap::new();

            headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

            let response = client
                .post(provider)
                .headers(headers)
                .json(&bodies)
                .send()
                .await;
            let response = match response {
                Ok(res) => res,
                Err(e) => {
                    println!("{:?}", e);
                    continue;
                }
            };

            match response.json().await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    println!("{:?}", e);
                    continue;
                }
            }
        }

        Err(custom_error)
    }

    pub fn build_body(&self, method: &str, params: Vec<String>) -> JsonBodyReq {
        let id = String::from("1");
        let jsonrpc = String::from("2.0");

        JsonBodyReq {
            params,
            id,
            jsonrpc,
            method: String::from(method),
        }
    }
}
