## Rust Server for ZilPay Wallet Tokens Metadata Storage and Configuration

This is a high-performance, multi-threaded server implemented in Rust. It is designed to store and configure metadata tokens for ZilPay wallet.
Running the Server

To run the server in development mode, use the following command:

```bash
cargo run
```

To build the server for production, use the following command:

```bash
cargo build --release
```
The following environment variable is required to run the server:

 * ACCESS_TOKEN: The access token to authenticate requests to the token updates endpoint.
 * DB_PATH: The path of the database filesystem.
 * PORT: The http server port. 

Configuration Files

The following configuration files are required to run the server:

   * CURRENCIES_KEY: The key to store the currencies database.
   * CURRENCIES_DATABASE: The directory to store the currencies database.
   * DEX_KEY: The key to store the liquidity pool data.
   * DEX_DATABASE: The directory to store the liquidity pool data.
   * META_KEY: The key to store the token metadata.
   * META_DATABASE: The directory to store the token metadata database.

Usage

### API Endpoints

The server provides the following API endpoints:

    GET /api/v1/rates: Returns the list of currencies.
    GET /api/v1/token/zlp: Returns the metadata for the ZLP token.
    PUT /api/v1/token/:base16: Updates the metadata by token address.
    GET /api/v1/dex: Returns the metadata for the ZLP token, the list of currencies, and the liquidity pool data.

Make sure to authenticate your requests using the ACCESS_TOKEN environment variable.
