use crate::config::zilliqa::{PROVIDERS, RPC_METHODS};

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
}
