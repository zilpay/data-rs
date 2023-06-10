pub const PROVIDERS: [&str; 3] = [
    "https://ssn.zilpay.io/api",
    "https://zilliqa.avely.fi/api",
    "https://api.zilliqa.com",
];
pub const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
pub const ZERO_ADDR: &str = "0000000000000000000000000000000000000000";
pub const HRP: &str = "zil";
pub struct RPCMethod {
    // Network-related methods
    pub get_network_id: &'static str,

    // Blockchain-related methods
    pub get_blockchain_info: &'static str,
    pub get_sharding_structure: &'static str,
    pub get_ds_block: &'static str,
    pub get_latest_ds_block: &'static str,
    pub get_num_ds_blocks: &'static str,
    pub get_ds_block_rate: &'static str,
    pub ds_block_listing: &'static str,
    pub get_tx_block: &'static str,
    pub get_latest_tx_block: &'static str,
    pub get_num_tx_blocks: &'static str,
    pub get_tx_block_rate: &'static str,
    pub tx_block_listing: &'static str,
    pub get_num_transactions: &'static str,
    pub get_transaction_rate: &'static str,
    pub get_current_mini_epoch: &'static str,
    pub get_current_ds_epoch: &'static str,
    pub get_prev_difficulty: &'static str,
    pub get_prev_ds_difficulty: &'static str,
    pub get_total_coin_supply: &'static str,
    pub get_miner_info: &'static str,

    // Transaction-related methods
    pub create_transaction: &'static str,
    pub get_transaction: &'static str,
    pub get_transaction_status: &'static str,
    pub get_recent_transactions: &'static str,
    pub get_transactions_for_tx_block: &'static str,
    pub get_transactions_for_tx_block_ex: &'static str,
    pub get_txn_bodies_for_tx_block: &'static str,
    pub get_txn_bodies_for_tx_block_ex: &'static str,
    pub get_num_txns_tx_epoch: &'static str,
    pub get_num_txns_ds_epoch: &'static str,
    pub get_minimum_gas_price: &'static str,

    // Contract-related methods
    pub get_contract_address_from_transaction_id: &'static str,
    pub get_smart_contracts: &'static str,
    pub get_smart_contract_code: &'static str,
    pub get_smart_contract_init: &'static str,
    pub get_smart_contract_state: &'static str,
    pub get_smart_contract_sub_state: &'static str,
    pub get_state_proof: &'static str,

    // Account-related methods
    pub get_balance: &'static str,
}

pub const RPC_METHODS: RPCMethod = RPCMethod {
    // Network-related methods
    get_network_id: "GetNetworkId",

    // Blockchain-related methods
    get_blockchain_info: "GetBlockchainInfo",
    get_sharding_structure: "GetShardingStructure",
    get_ds_block: "GetDsBlock",
    get_latest_ds_block: "GetLatestDsBlock",
    get_num_ds_blocks: "GetNumDSBlocks",
    get_ds_block_rate: "GetDSBlockRate",
    ds_block_listing: "DSBlockListing",
    get_tx_block: "GetTxBlock",
    get_latest_tx_block: "GetLatestTxBlock",
    get_num_tx_blocks: "GetNumTxBlocks",
    get_tx_block_rate: "GetTxBlockRate",
    tx_block_listing: "TxBlockListing",
    get_num_transactions: "GetNumTransactions",
    get_transaction_rate: "GetTransactionRate",
    get_current_mini_epoch: "GetCurrentMiniEpoch",
    get_current_ds_epoch: "GetCurrentDSEpoch",
    get_prev_difficulty: "GetPrevDifficulty",
    get_prev_ds_difficulty: "GetPrevDSDifficulty",
    get_total_coin_supply: "GetTotalCoinSupply",
    get_miner_info: "GetMinerInfo",

    // Transaction-related methods
    create_transaction: "CreateTransaction",
    get_transaction: "GetTransaction",
    get_transaction_status: "GetTransactionStatus",
    get_recent_transactions: "GetRecentTransactions",
    get_transactions_for_tx_block: "GetTransactionsForTxBlock",
    get_transactions_for_tx_block_ex: "GetTransactionsForTxBlockEx",
    get_txn_bodies_for_tx_block: "GetTxnBodiesForTxBlock",
    get_txn_bodies_for_tx_block_ex: "GetTxnBodiesForTxBlockEx",
    get_num_txns_tx_epoch: "GetNumTxnsTxEpoch",
    get_num_txns_ds_epoch: "GetNumTxnsDSEpoch",
    get_minimum_gas_price: "GetMinimumGasPrice",

    // Contract-related methods
    get_contract_address_from_transaction_id: "GetContractAddressFromTransactionID",
    get_smart_contracts: "GetSmartContracts",
    get_smart_contract_code: "GetSmartContractCode",
    get_smart_contract_init: "GetSmartContractInit",
    get_smart_contract_state: "GetSmartContractState",
    get_smart_contract_sub_state: "GetSmartContractSubState",
    get_state_proof: "GetStateProof",

    // Account-related methods
    get_balance: "GetBalance",
};
