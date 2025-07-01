/// Record a failed transaction with its txid and reason in /Json/Failures as a text file.
/// The file is named `{txid}-{reason}.txt` (reason sanitized for filename).
pub fn record_failed_transaction(txid: &str, reason: &str) {
    use std::fs;
    use std::io::Write;
    let failures_dir = "./Json/Failures";
    if !std::path::Path::new(failures_dir).exists() {
        let _ = fs::create_dir_all(failures_dir);
    }
    // Sanitize reason for filename (remove spaces and special chars)
    let sanitized_reason = reason.replace(" ", "_").replace("/", "_").replace("\\", "_");
    let filename = format!("{}/{}-{}.txt", failures_dir, txid, sanitized_reason);
    let content = format!("txid: {}\nreason: {}\ndate: {}\n", txid, reason, chrono::Local::now().to_rfc3339());
    if let Ok(mut file) = fs::File::create(&filename) {
        let _ = file.write_all(content.as_bytes());
    }
}
use chrono::Local;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use warp::reject::Reject;

use bitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn};
use bitcoin::consensus::deserialize;
use hex::decode;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandStruct {
    pub txid: String,
    pub payload: String,
    pub bid_payload: Option<Vec<BidPayload>>,
    pub contract_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RelayedCommandStruct {
    pub txid: String,
    pub payload: String,
    pub bid_payload: Option<Vec<BidPayload>>,
    pub contract_id: Option<String>,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct PendingCommandStruct {
    pub txid: String,
    pub payload: String,
    pub bid_payload: Option<Vec<BidPayload>>,
    pub contract_id: Option<String>,
    pub time_added: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BidPayload {
    pub contract_id: String,
    pub trade_txs: Vec<TradeTx>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TradeTx {
    pub order_id: String,
    pub accept_tx: String,
    pub fulfil_tx: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResultStruct {
    pub result: String,
}

// Stucts for Http client calls
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TxInfo {
    pub txid: Option<String>,
    pub vout: Option<Vec<Vout>>,
    pub vin: Option<Vec<Vin>>,
    pub status: Option<Status>,
    pub fee: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vout {
    pub scriptpubkey: Option<String>,
    pub scriptpubkey_asm: Option<String>,
    pub scriptpubkey_type: Option<String>,
    pub scriptpubkey_address: Option<String>,
    pub value: Option<u64>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vin {
    pub txid: String,
    pub vout: u32,
    pub prevout: Option<Vout>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Status {
    pub confirmed: Option<bool>,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractImport {
    pub contract_id: String,
    pub ticker: String,
    pub rest_url: String,
    pub contract_type: String,
    pub decimals: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BoundUtxoData {
    pub utxo: String,
    pub bind_type: i32,
    pub bound_data: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UtxoBalanceResult {
    pub balance_type: String,
    pub balance_value: String,
    pub contract_id: String,
    pub btc_price: Option<String>,
    pub num_bids: Option<String>,
    pub highest_bid: Option<String>,
    pub drip_amount: Option<u64>,
    pub min_bid: Option<String>,
    pub list_utxo: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CheckBalancesResult {
    pub balances: Vec<UtxoBalanceResult>,
    pub summaries: Vec<ContractSummary>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UtxoBalances {
    pub contract_ids: Vec<String>,
    pub utxos: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TxidCheck {
    pub contract_ids: Vec<String>,
    pub txids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TxidCheckResponse {
    pub contract_id: String,
    pub entries: Vec<ContractHistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TradeUtxoRequest {
    pub contract_id: String,
    pub utxos: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ListingSummary {
    pub quantity: u64,
    pub list_price: u64,
    pub bid_count: u64,
    pub highest_bid: u64,
    pub listing_utxo: String,
    pub pending_listing: bool,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContractListingResponse {
    pub contract_id: String,
    pub ticker: String,
    pub rest_url: String,
    pub contract_type: String,
    pub decimals: i32,
    pub listing_summaries: Vec<ListingSummary>,
}
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContractTradeResponse {
    pub contract_id: String,
    pub order_id: String,
    pub bid_utxo: String,
    pub listing_amount: u64,
    pub listing_price: u64,
    pub bid_amount: u64,
    pub bid_price: u64,
    pub listing_utxo: String,
    pub bid_pending: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractHistoryEntry {
    pub tx_type: String,
    pub scl_value: u64,
    pub txid: String,
    pub pending: bool,
    pub btc_price: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub block_height: i32,
    pub memes: Vec<String>,
    pub reserved_tickers: Option<Vec<String>>,
    pub hosts_ips: Option<Vec<String>>,
    pub my_ip_split: Option<Vec<u8>>,
    pub my_ip: Option<String>,
    pub key: Option<String>,
    pub esplora: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FulfilledSummary {
    pub bid_price: u64,
    pub listing_price: u64,
    pub listing_amount: u64,
    pub bid_amount: u64,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContractInteractions {
    pub fulfillment_summaries: Vec<FulfilledSummary>,
    pub total_transfers: u64,
    pub total_transfer_value: u64,
    pub total_burns: u64,
}

#[derive(Debug, Deserialize, Serialize, Default)]
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
    pub current_bids: u64,
    pub available_airdrops: Option<u64>,
    pub airdrop_amount: Option<u64>,
    pub max_supply: Option<u64>,
    pub lp_contracts: Option<(String, String)>,
    pub lp_ratio: Option<f64>,
    pub token_data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PagingMetaData {
    pub current_page: usize,
    pub total_pages: usize,
    pub page_entries: usize,
    pub entries: usize,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CancelRequest {
    pub contract_id: String,
    pub txid: String,
    pub utxo: String,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct BidData {
    pub bid_price: String,
    pub bid_amount: String,
    pub order_id: String,
    pub fulfill_tx: String,
    pub accept_tx: String,
    pub reseved_utxo: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Lookups {
    pub lps: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LiquidityPoolString {
    pub contract_id_1: String,
    pub contract_id_2: String,
    pub pool_1: String,
    pub pool_2: String,
    pub fee: String,
    pub k: String,
    pub liquidity_ratio: String,
    pub swaps: HashMap<String, (u64, u64)>,
    pub liquidations: HashMap<String, (u64, u64)>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpentResult {
    pub spent: bool,
}

#[derive(Debug)]
pub struct CustomError {
    pub message: String,
}

impl Reject for CustomError {}
impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// File Handling functions
pub fn read_from_file(relative_path: String) -> Option<String> {
    if !fs::metadata(&relative_path).is_ok() {
        return None;
    }

    let _data = match fs::read_to_string(&relative_path) {
        Ok(data) => return Some(data),
        Err(_) => return None,
    };
}

pub fn write_contract_directory(relative_path: String, data: String, contract_id: &str) -> bool {
    if !fs::metadata(format!("./Json/Contracts/{}", &contract_id)).is_ok() {
        let _ = fs::create_dir(format!("./Json/Contracts/{}", &contract_id));
    }

    match fs::write(&relative_path, data) {
        Ok(_) => return true,
        Err(_) => return false,
    }
}

pub fn write_to_file(relative_path: String, data: String) -> bool {
    match fs::write(&relative_path, data) {
        Ok(_) => return true,
        Err(_) => return false,
    }
}

pub fn enqueue_item(filename: String, item: &str) -> std::io::Result<()> {
    match fs::write(&filename, item) {
        Ok(_) => return Ok(()),
        Err(err) => return Err(err),
    }
}

pub fn dequeue_item(path: &str) -> Result<String, String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Err("Error".to_string()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => return Err("Error".to_string()),
        };

        let data = match fs::read_to_string(entry.path().clone()) {
            Ok(data) => data,
            Err(_) => return Err("Error".to_string()),
        };

        match fs::remove_file(entry.path().clone()) {
            Ok(_) => {}
            Err(_) => {}
        };
        return Ok(data);
    }

    return Err("Error".to_string());
}

pub fn read_queue(path: String) -> Result<Vec<(PendingCommandStruct, String)>, String> {
    let mut json_objects: Vec<(PendingCommandStruct, String)> = Vec::new();
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let data_str = match fs::read_to_string(entry.path()) {
                            Ok(data_str) => data_str,
                            Err(_) => continue,
                        };

                        let json_object: PendingCommandStruct =
                            match serde_json::from_str(&data_str) {
                                Ok(json_object) => json_object,
                                Err(_) => continue,
                            };

                        json_objects
                            .push((json_object, entry.path().to_string_lossy().to_string()));
                    }
                    Err(_) => return Err("Error reading directory".to_string()),
                }
            }
            Ok(json_objects)
        }
        Err(_) => return Err("Error reading directory".to_string()),
    }
}

pub fn trim_chars<'a>(input: &'a str, chars: &'a str) -> &'a str {
    let start = input.find(|c| !chars.contains(c)).unwrap_or(input.len());
    let end = input.rfind(|c| !chars.contains(c)).unwrap_or(0);
    &input[start..=end]
}

pub fn replace_payload_special_characters(input: &String) -> String {
    let mut trimmed_str = input.replace("[", "");
    trimmed_str = trimmed_str.replace("]", "");
    trimmed_str = trimmed_str.replace("}", "");
    trimmed_str = trimmed_str.replace(" ", "");
    trimmed_str
}

pub async fn handle_get_request(url: String) -> Option<String> {
    let client = Client::new();
    let response = client.get(&url).send().await;
    match response {
        Ok(response) => {
            if !response.status().is_success() {
                return None;
            }
            let body = response.text().await;
            match body {
                Ok(body_data) => return Some(body_data),
                Err(_) => return None,
            }
        }
        Err(_) => return None,
    }
}

pub fn extract_commands(payload: &str) -> Result<Vec<String>, String> {
    let re = match Regex::new(r"\{([^}]*)\}") {
        Ok(re) => re,
        Err(_) => return Err("Unable to find commands in payload".to_string()),
    };

    let matches: Vec<String> = re
        .captures_iter(payload)
        .filter_map(|capture| match capture.get(1) {
            Some(value) => Some(value.as_str().to_string()),
            None => None,
        })
        .collect();

    if matches.len() > 1 && payload.contains("SCL") {
        return Err("Mint command cannot be batched".to_string());
    }

    return Ok(matches);
}

pub fn extract_contract_id(payload: &str) -> Result<String, String> {
    let words: Vec<&str> = payload.split(":").collect();
    if words.len() == 0 {
        return Err("Contract id not found in the input string.".to_string());
    }
    let trimmed_str = &words[0].replace("{", "");
    return Ok(trimmed_str.to_string());
}

pub async fn check_utxo_inputs(utxos: &Vec<String>, txid: &str) -> bool {
    let tx_info: TxInfo = match get_transaction(txid, false).await {
        Ok(tx_info) => tx_info,
        Err(_) => return false,
    };
    //capture the senders for input validation
    let inputs = match &tx_info.vin {
        Some(v) => v,
        None => return false,
    };

    let mut input_str: Vec<String> = Vec::new();
    for input in inputs {
        let c = format!("{}:{}", &input.txid, &input.vout);
        input_str.push(c);
    }

    for u in utxos {
        if !&input_str.contains(&u) {
            return false;
        }
    }
    return true;
}

pub async fn get_transaction(txid: &str, update: bool) -> Result<TxInfo, String> {
    let path = format!("./Json/TXs/{}.txt", txid);
    if !fs::metadata(&path).is_ok() || update {
        let config = match read_server_config() {
            Ok(config) => config,
            Err(_) => Config::default(),
        };

        let esplora = match config.esplora {
            Some(esplora) => esplora,
            None => "https://btc.darkfusion.tech/".to_owned(),
        };

        let url = esplora + "tx/" + &txid;
        let response = match handle_get_request(url.clone()).await {
            Some(response) => response,
            None => {
                println!("No response from esplora: {}", url);
                return Err("No response from esplora".to_string());
            }
        };

        let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
            Ok(tx_info) => tx_info,
            Err(_) => return Err("No response from esplora".to_string()),
        };

        match serde_json::to_string(&tx_info) {
            Ok(tx_str) => write_to_file(path, tx_str),
            Err(_) => return Err("Failed to read lookups".to_string()),
        };

        return Ok(tx_info);
    } else {
        let data = match fs::read_to_string(&path) {
            Ok(data) => data,
            Err(err) => return Err(err.to_string()),
        };

        let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&data) {
            Ok(tx_info) => tx_info,
            Err(_) => return Err("Unable to read TX in lookup".to_string()),
        };

        return Ok(tx_info);
    }
}

pub fn remove_transaction(txid: &str) {
    let path = format!("./Json/TXs/{}.txt", txid);
    let _ = fs::remove_file(&path);
}

pub async fn check_utxo_spent(utxo: &str, esplora: &String) -> Result<bool, String> {
    let split: Vec<&str> = utxo.split(":").collect();
    if split.len() < 2 {
        return Err("Unable to get tx status from esplora repsonse".to_string());
    }
    let url = esplora.to_string() + "tx/" + split[0] + "/outspend/" + split[1];

    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => return Err("No response from espolra".to_string()),
    };

    match serde_json::from_str::<SpentResult>(&response) {
        Ok(result) => return Ok(result.spent),
        Err(err) => return Err(err.to_string()),
    };
}

pub async fn get_tx_inputs(txid: &str) -> Result<Vec<String>, String> {
    let tx_info: TxInfo = match get_transaction(txid, false).await {
        Ok(tx_info) => tx_info,
        Err(_) => return Err("Unable to get inputs for txid".to_string()),
    };

    //capture the senders for input validation
    let inputs = match &tx_info.vin {
        Some(v) => v,
        None => return Err("Unable to get inputs for txid".to_string()),
    };

    let mut input_str: Vec<String> = Vec::new();
    for input in inputs {
        let c = format!("{}:{}", &input.txid, &input.vout);
        input_str.push(c);
    }

    return Ok(input_str);
}

pub async fn check_txid_confirmed(txid: &str) -> Result<bool, String> {
    let config = match read_server_config() {
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let esplora = match config.esplora {
        Some(esplora) => esplora,
        None => "https://btc.darkfusion.tech/".to_owned(),
    };

    let url = esplora.to_string() + "tx/" + &txid;

    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => return Err("No response from espolra".to_string()),
    };

    let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
        Ok(tx_info) => tx_info,
        Err(err) => return Err(err.to_string()),
    };

    let status = match tx_info.status {
        Some(status) => status,
        None => return Err("Unable to get tx status from esplora repsonse".to_string()),
    };
    let confirmed = match status.confirmed {
        Some(confirmed) => confirmed,
        None => return Err("Unable to get tx status from esplora repsonse".to_string()),
    };

    return Ok(confirmed);
}

pub async fn get_current_block_height_from_esplora() -> Result<i32, String> {
    let config = match read_server_config() {
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let esplora = match config.esplora {
        Some(esplora) => esplora,
        None => "https://btc.darkfusion.tech/".to_owned(),
    };

    let url = esplora + "blocks/tip/height";
    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => {
            return Err("Can't get response from esplora about current block height".to_string())
        }
    };

    match serde_json::from_str::<i32>(&response) {
        Ok(block_height) => return Ok(block_height),
        Err(_) => {
            return Err("Can't get response from esplora about current block height".to_string())
        }
    };
}

pub async fn get_current_block_height() -> Result<i32, String> {
    let config = match read_server_config() {
        Ok(config) => config,
        Err(_) => return Err("Could not read config file".to_string()),
    };

    return Ok(config.block_height);
}

pub fn get_contract_header(contract_id: &str) -> Result<ContractImport, String> {
    let path = "./Json/Contracts/".to_string() + "/" + contract_id + "/header.txt";
    match read_from_file(path) {
        Some(contract_obj) => {
            match serde_json::from_str::<ContractImport>(&contract_obj) {
                Ok(parsed_data) => return Ok(parsed_data),
                Err(_) => return Err("Failed to deserialize contract".to_string()),
            };
        }
        None => return Err("Could not find contract header".to_string()),
    }
}

pub fn get_txid_from_hash(tx_hex: &String) -> Result<String, String> {
    let tx_bytes = match decode(tx_hex) {
        Ok(tx_bytes) => tx_bytes,
        Err(_) => return Err("Failed to decode hex".to_string()),
    };

    let transaction: Transaction = match deserialize(&tx_bytes) {
        Ok(transaction) => transaction,
        Err(_) => return Err("Failed to decode transaction".to_string()),
    };

    return Ok(transaction.txid().to_string());
}

pub fn get_utxos_from_hash(tx_hex: &String) -> Result<Vec<String>, String> {
    let tx_bytes = match decode(tx_hex) {
        Ok(tx_bytes) => tx_bytes,
        Err(_) => return Err("Failed to decode hex".to_string()),
    };

    let transaction: Transaction = match deserialize(&tx_bytes) {
        Ok(transaction) => transaction,
        Err(_) => return Err("Failed to decode transaction".to_string()),
    };

    let mut utxos: Vec<String> = Vec::new();
    let inputs: Vec<TxIn> = transaction.input;
    for input in inputs {
        let prev_output: OutPoint = input.previous_output;
        let utxo: String = format!("{}:{}", prev_output.txid, prev_output.vout);
        utxos.push(utxo);
    }

    return Ok(utxos);
}

pub fn read_server_config() -> Result<Config, String> {
    let path = format!("./Json/config.txt");
    if !fs::metadata(&path).is_ok() {
        return Err("Unable to read the server config file".to_string());
    }

    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    let parsed_data: Result<Config, _> = serde_json::from_str(&data);
    match parsed_data {
        Ok(data) => return Ok(data),
        Err(_) => return Err("Failed to deserialize contract".to_string()),
    }
}

pub fn save_server_config(config: Config) -> Result<String, String> {
    let path = format!("./Json/config.txt");
    match serde_json::to_string(&config) {
        Ok(config_str) => write_to_file(path, config_str),
        Err(_) => return Err("Failed to saved config".to_string()),
    };

    return Ok("Successfully saves config".to_string());
}

pub fn read_server_lookup() -> Result<Lookups, String> {
    let path = format!("./Json/lookups.txt");
    if !fs::metadata(&path).is_ok() {
        match serde_json::to_string(&Lookups::default()) {
            Ok(lookups_str) => write_to_file(path.clone(), lookups_str),
            Err(_) => return Err("Unable to read the server lookups file".to_string()),
        };
    }

    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    let parsed_data: Result<Lookups, _> = serde_json::from_str(&data);
    match parsed_data {
        Ok(data) => return Ok(data),
        Err(_) => return Err("Failed to deserialize contract".to_string()),
    }
}

pub fn save_server_lookup(lookups: Lookups) -> Result<String, String> {
    let path = format!("./Json/lookups.txt");
    match serde_json::to_string(&lookups) {
        Ok(lookups_str) => write_to_file(path, lookups_str),
        Err(_) => return Err("Failed to saved lookups".to_string()),
    };

    return Ok("Successfully saves lookups".to_string());
}

pub fn read_contract_interactions(contract_id: &str) -> Result<ContractInteractions, String> {
    let path = "./Json/Contracts/".to_string() + "/" + contract_id + "/interactions.txt";
    if !fs::metadata(&path).is_ok() {
        let interactions = ContractInteractions::default();
        _ = save_contract_interactions(&interactions, contract_id)
    }
    match read_from_file(path) {
        Some(contract_obj) => {
            let parsed_data: Result<ContractInteractions, _> =
                serde_json::from_str(&contract_obj.to_string());
            match parsed_data {
                Ok(data) => return Ok(data),
                Err(_) => return Err("Failed to deserialize contract interactions".to_string()),
            }
        }
        None => return Err("Could not find contract interactions".to_string()),
    }
}

pub fn save_contract_interactions(
    interactions: &ContractInteractions,
    contract_id: &str,
) -> Result<String, String> {
    let path = format!("{}/{}/interactions.txt", "./Json/Contracts/", contract_id);
    match serde_json::to_string(&interactions) {
        Ok(state_string) => write_to_file(path, state_string),
        Err(_) => return Err("Failed to save updated contract interactions".to_string()),
    };

    Ok("Success".to_string())
}

pub fn save_command_backup(command: &CommandStruct, pending: bool) {
    let formatted_date_time = Local::now().format("%Y-%m-%d").to_string();
    let mut path = format!("{}/{}.txt", "./Json/Backups", formatted_date_time);
    if pending {
        path = format!("{}/{}-pending.txt", "./Json/Backups", formatted_date_time);
    }
    let mut backups: HashMap<String, (String, Option<Vec<BidPayload>>, String)> = HashMap::new();
    if fs::metadata(&path).is_ok() {
        match read_from_file(path.clone()) {
            Some(backup_obj) => {
                let parsed_data: Result<
                    HashMap<String, (String, Option<Vec<BidPayload>>, String)>,
                    _,
                > = serde_json::from_str(&backup_obj.to_string());
                backups = match parsed_data {
                    Ok(data) => data,
                    Err(_) => return,
                }
            }
            None => return,
        }
    }

    backups.insert(
        command.txid.clone(),
        (
            command.payload.clone(),
            command.bid_payload.clone(),
            Local::now().format("%H:%M:%S").to_string(),
        ),
    );
    match serde_json::to_string(&backups) {
        Ok(state_string) => write_to_file(path, state_string),
        Err(_) => return,
    };
}

/// Given a list of UTXOs in the format "txid:vout", returns a map of UTXO -> address.
/// If a UTXO cannot be resolved, it will not be included in the result.
pub async fn get_addresses_for_utxos(utxos: Vec<String>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for utxo in utxos {
        let parts: Vec<&str> = utxo.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let txid = parts[0];
        let vout: usize = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Ok(tx_info) = get_transaction(txid, false).await {
            if let Some(vouts) = tx_info.vout {
                if let Some(vout_obj) = vouts.get(vout) {
                    if let Some(address) = &vout_obj.scriptpubkey_address {
                        result.insert(utxo.clone(), address.clone());
                    }
                }
            }
        }
    }
    result
}
