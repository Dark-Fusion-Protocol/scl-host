SCL Host
=====================================================================================================================================================================
This is an example of an SCL Host as well as a restful validation paradigm that can be used to run and host the contract using a Rust Console Application.

This is dependant on RUST being installed

Rust can be installed by following these instructions:

	https://www.rust-lang.org/tools/install

Or on Mac OS X using this curl command:

	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh 

<br>

Building the Console Application
=====================================================================================================================================================================
To build or run a new exectuable Rust must be installed.
- ## Build an executable
    - In the project directory run the following commands to build to the target directory from which you can run the executable file.
  
  - ``` cargo build ```
  <br>

- ## Compile and run the console application
  - In the project directory run the following commands to compile and run the console application.
 
  - ```  cargo run ```

<br>

Server Requests
=====================================================================================================================================================================

## Get Requests
- ## General requests
  - Health check: {URl}:{Port}/health
    - Responds with a 200 to show the server is healthy 
  - List of all contract on the server: {URl}:{Port}/contracts
    - Returns a list of contract ids as strings which are currentlly hosted on the server  
  
- ## Contracts Requests
- Contract requests have the general format **{URl}:{PORT}/{CONTRACT ID}/{FIELD}**. Where the **URL** is the IP/URL the server is being hosted on, the **PORT** is the port number the server is using to recieve requests, the **CONTRACT ID** is specific contract you wish to interact with and the **FIELD** is the data you wish to get from the contract.
  - Entire contract state : {URl}:{Port}/{Contract ID}/state
    - An SCL contract object will be returned depedning on the contract type
    -  https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/state
  - Import Contract header : {URl}:{Port}/{Contract ID}/import_contract
    - Returns a import contract object
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/import_contract
  - Contract ticker: {URl}:{Port}/{Contract ID}/ticker
      - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/ticker
  - Contract Contract ID: {URl}:{Port}/{Contract ID}/contractid
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/contractid
  - Contract total supply: {URl}:{Port}/{Contract ID}/supply
      - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/supply
  - Contract owners: {URl}:{Port}/{Contract ID}/owners
      - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/owners
  - Contract payloads: {URl}:{Port}/{Contract ID}/payloads
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/payloads
  - Contract decimals: {URl}:{Port}/{Contract ID}/decimals
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/decimals
  - Contract listings: {URl}:{Port}/{Contract ID}/listings
      - Returns a hashmap of listing objects with order ids as keys
      - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/listings

  - Contract bids: {URl}:{Port}/{Contract ID}/bids
    - Returns a hashmap of bid objects with bid ids as keys
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/bids

  - Contract fulfillments: {URl}:{Port}/{Contract ID}/fulfillments
    - Returns a hashmap of fulfillment objects with order ids as keys
    - https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/fulfillments
  
  - Contract Summary: {URl}:{Port}/{Contract ID}/summary
    -  Returns contract summary object with trade and contract information
    -  https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/summary
  
  - Contract History: {URl}:{Port}/{Contract ID}/history
    -  Returns a list of contract history entry object with information about each contract payload
    -  https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/history

  - Contract Trades: {URl}:{Port}/{Contract ID}/trades
    -  Returns a list of contract trades fulfilled
    -  https://testscl.darkfusion.tech/0be85cccfa15c58fc8544a862ba33bd6477cc91820d1735b1d9daf404a0cf7fc/trades
  
- Pending Contracts
    - Requests for pending commands can be done by preceding fields in the above contract requests with "pending-"
      - Example: Entire pending contract state : {URl}:{Port}/{Contract ID}/pending-state
  
- ## General Requests
-   - Contracts: {URl}:{Port}/contracts
    -  Returns a list of contract IDs hosted on the contract
    -  https://testscl.darkfusion.tech/contracts
  
  - Coin Drops: {URl}:{Port}/coin_drops
    -  Returns a list of contract summaris of the coin drop contracts
    -  https://testscl.darkfusion.tech/coin_drops
  
- ## UTXO Requests
- UTXO requests have the general format **{URl}:{PORT}/{CONTRACT ID}/{FIELD}/{UTXO}**. Where the **URL** is the IP/URL trhe server is being hosted on, the **PORT** is the port number the server is using to recieve requests, the **CONTRACT ID** is specific contract you wish to interact with and the **FIELD** is the data type you wish to get from the contract and the **UTXO** is the bound utxo which you want to retieved the data associated with.
  - SCL amount bound to utxo: {URl}:{Port}/{Contract ID}/owner/{utxo}
    - Returns an SCL amount bound to the UTXO as a number

  -  Listings for order ID: {URl}:{Port}/{Contract ID}/listing/{listing_utxo}
     - Returns a Listing object
  
  -  Bid for bid ID: {URl}:{Port}/{Contract ID}/bid/{bidding_utxo}
     - Returns a Bid object

  -  Bids for order ID: {URl}:{Port}/{Contract ID}/bids_on_listing/{listing_utxo}
     - Returns a HashMap of bid IDs and thier corresponsing Bid objects for a given listing

  -  Bids for order ID: {URl}:{Port}/{Contract ID}/bids_on_listing/{listing_utxo}
     - Returns a HashMap of bid IDs and thier corresponsing Bid objects for a given listing as well as the listing object

  -  Listing for bid utxo: {URl}:{Port}/{Contract ID}/listing_for_bid/{bidding_utxo}
     - Returns a HashMap of bid IDs and thier corresponsing Bid objects for a given listing
     
- Pending UTXOS
    - Requests for pending commands can be done by preceding fields in the above contract requests with "pending-"
      - Example: Entire pending contract state : {URl}:{Port}/{Contract ID}/pending-listing/{order_id_utxo}

## Post Requests
- ### Commands
  - Send payload to be processed: {URl}:{Port}/commands
    - Json body for post is a command struct which contains a txid string and a payload string
  
- ### Check If Utxos are bound
  - Send list of utxos to be checked if they are bound: {URl}:{Port}/check_utxos
    - Json body for the post is a CheckBalancesResult object
    - A list of UtxoBalanceResult objects will be return being the same length of the utxos sent, with the balance_type of  indicatting what it is bound to.
      - If the balance value is 0 the utxo is unbound
      - If the balance type starts with O- it means it is a bid and the SCL owner amount is contained in the balance_value
      - If the balance type starts with B- it means it is a bid and the SCL bid amount is contained in the balance_value
      - If the balance type starts with L- it means it is a listing and the SCL listing amount is contained in the balance_value
      - If the balance type starts with C- it means it is a claim of an airdrop and the SCL claim amount is contained in the balance_value
      - If the balance type starts it P- its means it is pending eg : P-O- or P-B

- ### Contract summaries
  - Send list of contract_ids to get summaries for the contracts: {URl}:{Port}/summaries
    - Json body for the post is a list of contract_ids
    - A list of contract summary object with trade and contract information will be return being the same length of the contract ids sent

- ### Listing summaries
  - Send list of contract_ids and listing utxos to get summaries for the contracts: {URl}:{Port}/listing_summaries
    - Json body for the post is a list of trade utxo request objects
    - A list of contract trade response response objects with trade information will be returned

- ### Listing Trade summaries
  - Send list of contract_ids and bid utxos to get summaries for the contracts: {URl}:{Port}/bid_utxo_trade_info
    - Json body for the post is a list of trade utxo request objects
    - A list of contract lising response objects with listing and contract information will be returned

- ### Txid contract history
  - Send list of contract_ids and order ids to get summaries for the contracts: {URl}:{Port}/check_txids_history
    - Json body for the post is a cancel request object
    - A list of txid check response objects with payload  information will be returned
  
<br>

  ## Sending Data Types
  ```
    pub struct CommandStruct {
    pub txid: String,
    pub payload: String,
  }

  pub struct UtxoBalances {
    pub contract_ids: Vec<String>,
    pub utxos: Vec<String>,
  }

  pub struct TradeUtxoRequest{
    pub contract_id: String,
    pub utxos: Vec<String>,
  }

  pub struct TxidCheck{
    pub contract_ids: Vec<String>,
    pub txids: Vec<String>,
}

pub struct CancelRequest{
    pub contract_id: String,
    pub txid: String,
    pub utxo: String,
}

```
  
## Recieving Data Types
``` 
pub struct SCL01Contract {
    pub ticker: String,
    pub contractid: String,
    pub supply: u64,
    pub decimals: i32,
    pub owners: HashMap<String, u64>,
    pub payloads: HashMap<String, String>,
    pub listings : Option<HashMap<String, Listing>>,
    pub bids : Option<HashMap<String, Bid>>,
    pub fulfillments : Option<HashMap<String, String>>,
    pub drips: Option<HashMap<String,Vec<Drip>>>,
    pub diminishing_airdrops: Option<HashMap<String,DimAirdrop>>,
    pub dges: Option<HashMap<String, DGE>>,
    pub airdrop_amount: Option<u64>,
    pub total_airdrops: Option<u64>,
    pub current_airdrops: Option<u64>,
    pub pending_claims: Option<HashMap<String, u64>>,
    pub last_airdrop_split: Option<Vec<String>>,
    pub right_to_mint: Option<HashMap<String, u64>>,
    pub max_supply: Option<u64>
}

  HashMap<String, Listing>

  HashMap<String, Bid>

  HashMap<String, String>

pub struct Listing {
    pub list_utxo: String,
    pub list_amt: u64,
    pub price: u64,
    pub rec_addr: String,
    pub change_utxo:String,
    pub valid_bid_block: Option<i32>
}

pub struct Bid {
    pub bid_price: u64,
    pub bid_amount: u64,
    pub order_id: String,
    pub fulfill_tx: String,
    pub accept_tx: String,
    pub reseved_utxo:String,
    pub fullfilment_utxos: Vec<String>
}

pub struct Drip {
    pub block_end: u64,
    pub drip_amount: u64,
    pub amount: u64,
    pub start_block: u64,
    pub last_block_dripped: u64
}

pub struct DimAirdrop {
    pub pool_amount: u64,
    pub step_down_amount: u64,
    pub step_period_amount: u64,
    pub max_airdrop: u64,
    pub min_airdrop: u64,
    pub current_airdrop: u64,
    pub current_in_period: u64,
    pub amount_airdropped: u64,
    pub last_airdrop_split: Option<Vec<String>>,
    pub single_drop: bool,
    pub claimers: HashMap<String, u64>

}

pub struct DGE {
    pub pool_amount: u64,
    pub sats_rate: u64,
    pub max_drop: u64,
    pub current_amount_dropped: u64,
    pub donations_address: String,
    pub drip_duration: u64,
    pub single_drop: bool,
    pub donaters: HashMap<String, u64>
}

  pub struct ContractImport {
    pub contract_id: String,
    pub ticker: String,
    pub rest_url: String,
    pub contract_type: String,
    pub decimals: i32
}

  pub struct BoundUtxoData {
    pub bind_type: i32,
    pub bound_data: String,
  }

  pub struct ContractSummary {
    pub contract_id: String,
    pub ticker: String,
    pub rest_url: String,
    pub contract_type: String,
    pub decimals: i32,
    pub supply: u64,
    pub total_owners: u64,
    pub average_listing_price: u64,
    pub average_traded_price: u64,
    pub total_traded: u64,
    pub total_listed: u64,
    pub contract_interactions: u64,
    pub total_transfers: u64,
    pub total_burns: u64,
    pub current_listings: u64,
    pub current_bids: u64
  }

  pub struct ContractHistoryEntry{
    pub tx_type: String,
    pub scl_value: u64,
    pub txid: String,
    pub pending: bool,
    pub btc_price: Option<u64>,
  }

  
  pub struct UtxoBalanceResult {
    pub balance_type: String,
    pub balance_value: u64,
    pub contract_id: String,
    pub btc_price: Option<u64>,
  }

  pub struct CheckBalancesResult{
    pub balances: Vec<UtxoBalanceResult>,
    pub summaries: Vec<ContractSummary>
}

  
  pub struct ContractListingResponse {
    pub contract_id: String,
    pub ticker: String,
    pub rest_url: String,
    pub contract_type: String,
    pub decimals: i32,
    pub listing_summaries: Vec<ListingSummary>
  }

  pub struct TxidCheckResponse{
    pub contract_id: String,
    pub entries: Vec<ContractHistoryEntry>,
  }
  
  pub struct ContractTradeResponse {
    pub contract_id: String,
    pub order_id: String,
    pub bid_utxo: String,
    pub listing_amount: u64,
    pub listing_price: u64,
    pub bid_amount: u64,
    pub bid_price: u64,
  }
  ```

