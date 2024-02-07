use chrono::{Local, NaiveDateTime};
use crypto_hash::{hex_digest, Algorithm};
use reqwest::{header, Client};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::path::Path;
use tokio::time::{Duration, Instant};
use warp::{reject, Filter, Rejection, Reply};
extern crate regex;
use std::env;

mod scl01{pub(crate) mod scl01_contract;pub(crate) mod scl01_utils;}
use scl01::scl01_contract::{self, SCL01Contract};
use crate::scl01::scl01_utils::{self, perform_minting_scl01, perform_transfer_scl01, perform_burn_scl01, perform_list_scl01, perform_bid_scl01, perform_accept_bid_scl01, perform_fulfil_bid_scl01,perform_drips_scl01, perform_airdrop, perform_airdrop_split, convert_old_contracts, save_contract_scl01, read_contract_scl01,handle_payload_extra_trade_info, perform_listing_cancel_scl01, perform_bid_cancel_scl01,perform_drip_start_scl01};
use scl01::scl01_utils::{perform_create_diminishing_airdrop_scl01, perform_claim_diminishing_airdrop_scl01, perform_create_dge_scl01, perform_claim_dge_scl01, perform_minting_scl03, perform_rights_to_mint, perform_minting_scl02};

mod utils;
use utils::{check_txid_confirmed, dequeue_item, enqueue_item, extract_commands, extract_contract_id, get_contract_header, get_current_block_height, get_txid_from_hash, handle_get_request, read_contract_interactions, read_from_file, read_queue, read_server_config, save_command_backup, save_contract_interactions, save_server_config, trim_chars, write_to_file, BidData, RelayedCommandStruct};
use utils::{CheckBalancesResult,CommandStruct, CustomError, ResultStruct,TxInfo, PendingCommandStruct, UtxoBalanceResult, UtxoBalances, BidPayload,Config,ContractSummary,TxidCheck,ContractHistoryEntry, ListingSummary, ContractListingResponse,TxidCheckResponse, TradeUtxoRequest, ContractTradeResponse,PagingMetaData};

static TXCOMMANDSPATH: &'static str = "./Json/Queues/Confirmed/";
static PENDINGCOMMANDSPATH: &'static str = "./Json/Queues/Pending/";
static CONTRACTSPATH: &'static str = "./Json/Contracts/";
static QUEUESPATH: &'static str = "./Json/Queues/";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
     if args.len() > 1 {
        // The first argument after the program name is at index 1
        let user_input = &args[1];

        if user_input == "convert" {
            convert_old_contracts();
        }
    }
    
    let perform_command = warp::post()
        .and(warp::path("commands"))
        .and(warp::body::json())
        .and_then(handle_command_request);

    let perform_relay = warp::post()
        .and(warp::path("relay_commands"))
        .and(warp::body::json())
        .and_then(handle_relayed_command_request);

    let consolidate = warp::post()
        .and(warp::path("consolidate"))
        .and(warp::body::json())
        .and_then(handle_rebind);

    let check_utxos = warp::post()
        .and(warp::path("check_utxos"))
        .and(warp::body::json())
        .and_then(handle_check_utxo_files);

    let check_summaries = warp::post()
    .and(warp::path("summaries"))
    .and(warp::body::json())
    .and_then(handle_check_contract_summaries_request);

     let check_all_summaries = warp::get()
    .and(warp::path("all_summaries"))
    .and_then(handle_check_all_contract_summaries_request);

    let listing_summaries = warp::post()
        .and(warp::path("listing_summaries"))
        .and(warp::body::json())
        .and_then(handle_listing_summaries_request);

    let listings_for_bids = warp::post()
        .and(warp::path("bid_utxo_trade_info"))
        .and(warp::body::json())
        .and_then(handle_listings_for_bids_request);

    let txid_history = warp::post()
        .and(warp::path("check_txids_history"))
        .and(warp::body::json())
        .and_then(handle_check_txid_request);

    let get_contract_field = warp::get()
        .and(warp::path!(String / String))
        .and_then(handle_get_contract_field);

    let get_contract_field_paged = warp::get()
        .and(warp::path!(String / String / "page" / String))
        .and_then(handle_get_contract_field_paged);

    let get_utxo_data = warp::get()
        .and(warp::path!(String / String/ String))
        .and_then(handle_get_utxo_data);

    let get_contract_history = warp::get()
        .and(warp::path!(String / "history"))
        .and_then(handle_get_tx_history);

    let get_health = warp::get()
        .and(warp::path("health"))
        .and_then(handle_get_health);

    let get_contracts_route = warp::get()
        .and(warp::path("contracts"))
        .and_then(handle_get_contracts);

    let get_coin_drops = warp::get()
        .and(warp::path("coin_drops"))
        .and_then(handle_coin_drop_request);

    let get_meme_contracts = warp::get()
        .and(warp::path("meme_mints"))
        .and_then(handle_coin_drop_request);

    // Check server directories and files
    if !fs::metadata("./Json").is_ok() {
        fs::create_dir("./Json").expect("Failed to create Json directory");
    }
    if !fs::metadata("./Json/Backups").is_ok() {
        fs::create_dir("./Json/Backups").expect("Failed to create Backup directory");
    }
    if !fs::metadata("./Json/UTXOS").is_ok() {
        fs::create_dir("./Json/UTXOS").expect("Failed to create Backup directory");
    }
    if !fs::metadata(&QUEUESPATH).is_ok() {
        fs::create_dir(&QUEUESPATH).expect("Failed to create Queue directory");
    }
    if !fs::metadata(&CONTRACTSPATH).is_ok() {
        fs::create_dir("./Json/Contracts").expect("Failed to create Contracts directory");
    }
    if !fs::metadata(&TXCOMMANDSPATH).is_ok() {
        fs::create_dir(&TXCOMMANDSPATH).expect("Failed to create confirmed commands directory");
    }
    if !fs::metadata(&PENDINGCOMMANDSPATH).is_ok() {
        fs::create_dir(&PENDINGCOMMANDSPATH).expect("Failed to create pending commands directory");
    }
    if !fs::metadata("./Json/Queues/Claims").is_ok() {
        fs::create_dir("./Json/Queues/Claims").expect("Failed to create Claims directory");
    }

    if !fs::metadata(&"./Json/config.txt").is_ok() {
        let c = Config{block_height:0,memes:Vec::new(),sorting:false,sort_again:false,reserved_tickers:None, hosts_ips: None, my_ip_split: Some(vec![127,0,0,1]), my_ip: None, key: None, esplora: Some("https://btc.darkfusion.tech/".to_owned()), url: Some("https://scl.darkfusion.tech/".to_string()) };
        let _ = save_server_config(c);
    }

    // Create a warp filter that includes both the GET and POST routes
    let routes = perform_command
        .or(check_utxos)
        .or(check_summaries)
        .or(listings_for_bids)
        .or(txid_history)
        .or(listing_summaries)
        .or(get_contract_field)
        .or(get_contract_field_paged)
        .or(get_health)
        .or(get_contracts_route)
        .or(get_coin_drops)
        .or(get_utxo_data)
        .or(get_contract_history)
        .or(get_meme_contracts)
        .or(check_all_summaries)
        .or(consolidate)
        .or(perform_relay)
        .recover(handle_custom_rejection)
        .with(warp::cors()
            .allow_methods(vec!["GET", "POST", "OPTIONS"]) // Only allow the methods your server supports
            .allow_headers(vec!["Content-Type", "access-control-allow-methods", "access-control-allow-origin", "authorization", "cache-control", "x-xsrf-token",])
            .allow_any_origin() // Allow requests from any origin (for development/testing)
            .allow_credentials(false), // You may set this to true if needed
        );

    let mut pending_start_time = Instant::now();
    let pending_target_duration = Duration::from_secs(5);
    let _payload = tokio::spawn(async move {
        loop {
            let item_string = match dequeue_item(TXCOMMANDSPATH) {
                Ok(item_string) => item_string,
                Err(_) => continue,
            };
            
            println!("perform scl confirmed payload");
            let command: CommandStruct = match serde_json::from_str(&item_string){
                Ok(command) => command,
                Err(_) => continue, 
            };

            let config = match read_server_config(){
                Ok(config) => config,
                Err(_) => Config::default(),
            };
        
            let esplora = match config.esplora{
                Some(esplora) => esplora,
                None => "https://btc.darkfusion.tech/".to_owned(),
            };

            perform_commands(command.txid.as_str(), command.payload.as_str(), &command.bid_payload, false, &esplora).await;
        }
    });

    let _pending = tokio::spawn(async move {
        loop {
            let elapsed_time: Duration = Instant::now().duration_since(pending_start_time);
            if elapsed_time >= pending_target_duration {
                println!("handle pending scl payloads");
                great_sort().await;
                // Reset the timer for the next interval.
                pending_start_time = Instant::now();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => return,
    };

    let ip_split = match config.my_ip_split{
        Some(ip_split) => ip_split,
        None => return,
    };

    if ip_split.len() < 4 {
        return
    }

    //Start the server on port 8080
    warp::serve(routes).run(([ip_split[0], ip_split[1], ip_split[2], ip_split[3]], 8080)).await;
}

async fn handle_rebind(req: CommandStruct) -> Result<impl Reply, Rejection> {
    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let esplora = match config.esplora{
        Some(esplora) => esplora,
        None => "https://btc.darkfusion.tech/".to_owned(),
    };

    let url = format!("{}tx/{}",esplora, &req.txid);
    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => return Err(reject::custom(CustomError{ message : "Unable to check esplora".to_string()}))
    };

    let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
        Ok(tx_info) => tx_info,
        Err(_) => return Err(reject::custom(CustomError{ message : "Unable to deserialize txinfo".to_string()})),
    };
      let vout = match tx_info.vout {
        Some(vout) => vout,
        None => return Err(reject::custom(CustomError{ message : "INVALID OUTPUTS".to_string()})),
    };

    let mut op_return_flag = false;
     for output in vout {
        let scriptpubkey_type = match output.scriptpubkey_type {
            Some(scriptpubkey_type) => scriptpubkey_type,
            None => return Err(reject::custom(CustomError{ message : "INVALID SCRIPTPUBKEYTYPE".to_string()})),
        };
        if scriptpubkey_type == "op_return".to_string() {
            op_return_flag = true;
            break;
        }
    }
    if op_return_flag {
        return Err(reject::custom(CustomError{ message : "OP_RETURN FOUND, INVALID REBIND".to_string()}));
    }

    let vin = match tx_info.vin {
        Some(vin) => vin,
        None => return Err(reject::custom(CustomError{ message : "Unable to fetch inputs".to_string()})),
    };
       let status = match tx_info.status {
            Some(status) => status,
            None =>  return Err(reject::custom(CustomError{ message : "Unable to fetch tx status".to_string()})), 
        };
        let confirmed = match status.confirmed {
            Some(confirmed) => confirmed,
            None =>  return Err(reject::custom(CustomError{ message : "Unable to fetch tx status confirmed".to_string()})), 
        };
    if vin.len() == 0 {
        return Err(reject::custom(CustomError{ message : "transaction has has no inputs".to_string()}));
    }
    let mut contract = match read_contract_scl01(&req.payload, false){
        Ok(contract) => contract,
        Err(_) =>  return Err(reject::custom(CustomError{ message : "unable to read contract".to_string()})),
    };
    let mut senders: Vec<String> = Vec::new();
    for v in &vin {
        let utxo = format!("{}:{}", v.txid.clone(), v.vout.clone());
        senders.push(utxo);
    }
      let block_height = match get_current_block_height().await {
            Ok(block_height) => block_height,
            Err(_) => return Err(reject::custom(CustomError{ message : "Failed to get block height".to_string()})),
        };
    let mut rec = Vec::new();
    rec.push(format!("{}:0", &req.txid.clone())); 
     let drip = match contract.consolidate(&req.txid.clone(), &"CONSOLIDATE".to_string(), &senders, &rec, block_height as u64) {
        Ok(res) => res,
        Err(_) => return Err(reject::custom(CustomError{ message : "Failed to consolidate".to_string()})),
    };

    match save_contract_scl01(&contract,  &"CONSOLIDATE".to_string(), &req.txid.clone(), true) {
        Ok(_) => {},
        Err(_) => {}
    };
    if confirmed {
        for s in &senders{
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {} 
            }
        }

        let mut data = format!("{}:O-,{}", &contract.contractid, drip.1.clone());
        if drip.0 {
           data= format!("{}:DO-,{}", &contract.contractid,drip.1); 
        }

        match fs::write(format!("./Json/UTXOS/{}:0.txt", &req.txid),data.clone(),){
            Ok(_) => {},
            Err(_) => return Err(reject::custom(CustomError{ message : "Failed to write to file".to_string()})),
        };
        
        let _ = save_contract_scl01(&contract, &"CONSOLIDATE".to_string(),  &req.txid, false);

        let mut interactions =  match read_contract_interactions(&contract.contractid) {
            Ok(interactions) => interactions,
            Err(_) => return Err(reject::custom(CustomError{ message : "Failed to read interactions".to_string()})),
        };

        interactions.total_transfers += 1;
    
        interactions.total_transfer_value += drip.1.clone();
        match save_contract_interactions(&interactions, &contract.contractid) {
            Ok(_) => interactions,
            Err(_) => return Err(reject::custom(CustomError{ message : "Failed to save interactions".to_string()})),
        };
    } else {
             let mut data = format!("{}:O-,{}", &contract.contractid, drip.1.clone());
            if drip.0 {
                data= format!("{}:DO-,{}", &contract.contractid,drip.1); 
            }
             match fs::write(format!("./Json/UTXOS/{}:0.txt", &req.txid),data.clone(),){
                Ok(_) => {},
                Err(_) => return Err(reject::custom(CustomError{ message : "Failed to write utxo data".to_string()})), 
            };
    }
    
    let result = ResultStruct {
        result: format!("Successfully rebound SCL assets").to_string(),
    };

    return Ok(warp::reply::json(&result));
}

// Warp post route functions
async fn handle_command_request(req: CommandStruct) -> Result<impl Reply, Rejection> {
    let res = payload_validation_and_confirmation(req.txid.as_str(), req.payload.as_str()).await;
    if !res.0 || !res.1 {     
        let current_date_time = Local::now();
        let formatted_date_time = current_date_time.format("%Y-%m-%d %H:%M:%S").to_string();
        let pending_command = PendingCommandStruct{
            txid: req.txid.clone(),
            payload: req.payload.clone(),
            time_added: formatted_date_time,
            bid_payload: req.bid_payload.clone(),
        };

        let command_str = match serde_json::to_string(&pending_command) {
            Ok(command_str) => command_str,
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to serialize command data".to_string()}))
        };
    
        let mut path = format!("{}{}-{}.txt", PENDINGCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), &req.txid);
        if req.payload.contains("CLAIM_DIMAIRDROP") {
            path = format!("./Json/Queues/Claims/{}.txt", &req.txid);
        }

        let _ = match enqueue_item(path, &command_str.to_string()) {
            Ok(_) => {},
            Err(_) => return Err(reject::custom(CustomError { message: "Unable to add pending command to queue".to_string()})),
        };

        save_command_backup(&req, true);
    } else {       
        let command_str = match serde_json::to_string(&req) {
            Ok(command_str) => command_str,
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to serialize command data".to_string()}))
        };
        
        let _ = match enqueue_item(format!("{}{}-{}.txt", TXCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), req.txid.clone()), &command_str.to_string()) {
            Ok(_) => {},
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to queue data".to_string()}))
        };

        save_command_backup(&req, false);
    }

    let result = ResultStruct {
        result: format!("Successfully added payload to queue").to_string(),
    };

    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => return Ok(warp::reply::json(&result)),
    };

    let host_ips: Vec<String> = match config.hosts_ips {
        Some(host_ips) => host_ips,
        None => return Ok(warp::reply::json(&result)),
    };

    let my_ip = match config.my_ip {
        Some(my_ip) => my_ip,
        None => return Ok(warp::reply::json(&result)),
    };

    let key = match config.key {
        Some(key) => key,
        None => return Ok(warp::reply::json(&result)),
    };

    let realy_command: RelayedCommandStruct = RelayedCommandStruct{
        txid: req.txid,
        payload: req.payload,
        bid_payload: req.bid_payload,
        key: key,
    };

    let relay_command_str = match serde_json::to_string(&realy_command) {
        Ok(relay_command_str) => relay_command_str,
        Err(_) => return Ok(warp::reply::json(&result))
    };

    for ip in host_ips {
        if ip != my_ip {
            let url: String = format!("{}/relay_command", ip);
            println!("{}", url);
            let client = Client::new();
			match client
				.post(url)
				.header(header::CONTENT_TYPE, "application/json")
				.body(relay_command_str.clone())
				.send()
				.await{
					Ok(_) => println!("Successfully relayed"),
					Err(_) => println!("Failed to relay"),
			}	
		}
    }

    return Ok(warp::reply::json(&result));
}

async fn handle_relayed_command_request(req: RelayedCommandStruct) -> Result<impl Reply, Rejection> { 
    println!("Relayed command Recieved");
    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => return Err(reject::custom(CustomError{ message : "Unable to serialize config data".to_string()})),
    };

    if config.key != Some(req.key) {
        return Err(reject::custom(CustomError{ message : "Invalid Key".to_string()}))
    }

    let command = CommandStruct{
        txid: req.txid.clone(),
        payload: req.payload.clone(),
        bid_payload: req.bid_payload.clone(),
    };
    
    let res = payload_validation_and_confirmation(req.txid.as_str(), req.payload.as_str()).await;
    if !res.0 || !res.1 {     
        let current_date_time = Local::now();
        let formatted_date_time = current_date_time.format("%Y-%m-%d %H:%M:%S").to_string();
        let pending_command = PendingCommandStruct{
            txid: req.txid.clone(),
            payload: req.payload.clone(),
            time_added: formatted_date_time,
            bid_payload: req.bid_payload.clone(),
        };

        let command_str = match serde_json::to_string(&pending_command) {
            Ok(command_str) => command_str,
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to serialize command data".to_string()}))
        };
    
        let mut path = format!("{}{}-{}.txt", PENDINGCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), &req.txid);
        if req.payload.contains("CLAIM_DIMAIRDROP") {
            path = format!("./Json/Queues/Claims/{}.txt", &req.txid);
        }

        let _ = match enqueue_item(path, &command_str.to_string()) {
            Ok(_) => {},
            Err(_) => return Err(reject::custom(CustomError { message: "Unable to add pending command to queue".to_string()})),
        };

        save_command_backup(&command, true);
    } else {       
        let command_str = match serde_json::to_string(&command) {
            Ok(command_str) => command_str,
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to serialize command data".to_string()}))
        };
        
        let _ = match enqueue_item(format!("{}{}-{}.txt", TXCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), req.txid.clone()), &command_str.to_string()) {
            Ok(_) => {},
            Err(_) => return Err(reject::custom(CustomError{ message : "Unable to queue data".to_string()}))
        };

        save_command_backup(&command, false);
    }

    let result = ResultStruct {
        result: format!("Successfully added payload to queue").to_string(),
    };
    return Ok(warp::reply::json(&result));
}

async fn handle_check_utxo_files(data: UtxoBalances) -> Result<impl Reply, Rejection> {
    let current_block = match get_current_block_height().await{
        Ok(current_block) => current_block as u64,
        Err(_) => 0
    };
    let mut results:Vec<UtxoBalanceResult> = Vec::new();
    for utxo in data.utxos.clone(){
           let state_str = match read_from_file(format!("./Json/UTXOS/{}.txt", utxo)) {
                Some(state_str) => state_str,
                None => "unbound".to_string(),
            };

        if state_str.contains("unbound") {
            results.push(UtxoBalanceResult { balance_value: 0.to_string(), balance_type: "{\"Result\":\"UTXO specified is unbound.\"}".to_string(), contract_id: "".to_string(), btc_price: None,num_bids: None, highest_bid: None, drip_amount:None, min_bid: None, list_utxo: None});
        } else {
            let split: Vec<_> = state_str.split(":").collect();
            if split.len() < 2 {
                continue;
            }

            let contract_id = split[0];
            let second_split: Vec<_> = split[1].split(",").collect();
            if second_split.len() < 2 {
                continue;
            }

            let mut balance_type: String = second_split[0].to_string();
            let balance_value = second_split[1];
            let mut btc_price = 0;
            if second_split.len() >= 3 {
                btc_price = match  second_split[2].parse::<u64>(){
                    Ok(price) => price,
                    Err(_) => 0,
                };
            }

            let mut num_bids = 0;
            let mut highest_bid = 0;
            let mut min_bid = 0;
            let mut list_utxo: String = "".to_string();
            if second_split.len() >= 6 {
                num_bids = match second_split[3].parse::<u64>(){
                    Ok(price) => price,
                    Err(_) => 0,
                };

                highest_bid = match second_split[4].parse::<u64>(){
                    Ok(price) => price,
                    Err(_) => 0,
                };
                min_bid = match second_split[5].parse::<u64>(){
                    Ok(price) => price,
                    Err(_) => 0,
                };
            }

            if second_split.len() >= 7 {
                 list_utxo = second_split[6].to_string();
            }

            if balance_type.contains("B") && second_split.len() >= 5  {
                if split.len() < 3 {
                    continue;
                }

                list_utxo = format!("{}:{}",second_split[4].to_string(),split[2]);
            }

            if balance_type.contains("D") {
                balance_type = balance_type.replace("D", "");
                let pending =  balance_type.contains("P");
                let contract = match read_contract_scl01(contract_id, pending){
                    Ok(contract) => contract,
                    Err(_) => continue,
                };

                let all_drips = match contract.drips{
                    Some(all_drips) => all_drips,
                    None => continue,
                };

                let drips = match all_drips.get(&utxo){
                    Some(drips) => drips,
                    None => continue,
                };

                let mut drip_amount = 0;
                for drip in  drips {
                    drip_amount += (drip.block_end - current_block)* drip.drip_amount
                }

                results.push(UtxoBalanceResult {balance_type: balance_type.to_string(), balance_value: balance_value.to_string(), contract_id:contract_id.to_string(), btc_price: Some(btc_price.to_string()), num_bids:Some(num_bids.to_string()), highest_bid: Some(highest_bid.to_string()), drip_amount: Some(drip_amount), min_bid: Some(min_bid.to_string()), list_utxo:Some(list_utxo.to_string())});  
            }else{
                results.push(UtxoBalanceResult {balance_type: balance_type.to_string(), balance_value: balance_value.to_string(), contract_id:contract_id.to_string(), btc_price: Some(btc_price.to_string()), num_bids:Some(num_bids.to_string()), highest_bid: Some(highest_bid.to_string()), drip_amount: None, min_bid: Some(min_bid.to_string()), list_utxo:Some(list_utxo.to_string()) }); 
            }
        }
    }

    let mut summaries:Vec<ContractSummary> = Vec::new();
    for con in data.contract_ids.clone() {
        let s = match get_contract_field(&con, &"summary".to_string(), false, 1) {
            Ok(result) => result,
            Err(_) => String::new(),
        };

        match serde_json::from_str::<ContractSummary>(&s){
            Ok(result) => {
                summaries.push(result);
            },
            Err(_) => continue,
        };
    }

    let res = CheckBalancesResult{
        balances: results,
        summaries: summaries,
    };
    
    return Ok(warp::reply::json(&res));
}   

async fn handle_check_all_contract_summaries_request() -> Result<impl Reply, Rejection> {
    let mut results = "[".to_string();
    let contracts = match get_contracts() {
        Ok(c) => c,
        Err(_) =>return Err(warp::reject::custom(CustomError {message: "Unable to get contracts".to_string(),})),
    };

    for (index, contract_id) in contracts.iter().enumerate() {
        match get_contract_field(contract_id, &"summary".to_string(), false, 1) {
            Ok(result) => {
                if index > 0 && results.len() != 1 {
                    results.push_str(",");
                }
                results.push_str(&result);
            }
            Err(_) => continue,
        }
    }

    results.push_str("]");
    return Ok(warp::reply::html(results));
}

async fn handle_check_contract_summaries_request(mut contract_ids: Vec<String>) -> Result<impl Reply, Rejection> {
    let mut results = "[".to_string();
    contract_ids.truncate(5000);
    for (index, contract_id) in contract_ids.iter().enumerate() {
        match get_contract_field(contract_id, &"summary".to_string(), false, 1) {
            Ok(result) => {
                if index > 0 && results.len() != 1 {
                    results.push_str(",");
                }

                results.push_str(&result);
            }
            Err(_) => continue,
        }
    }

    results.push_str("]");
    return Ok(warp::reply::html(results));
}

async fn handle_coin_drop_request() -> Result<impl Reply, Rejection> {
    let mut results = "[".to_string();
    let contract_ids = match read_server_config(){
        Ok(config) => config.memes,
        Err(_) => Vec::new(),
    };

    for (index, contract_id) in contract_ids.iter().enumerate() {
        match get_contract_field(contract_id, &"summary".to_string(), false, 1) {
            Ok(result) => {
                if index > 0 && results.len() != 1 {
                    results.push_str(",");
                }

                results.push_str(&result);
            }
            Err(_) => continue,
        }
    }

    results.push_str("]");
    return Ok(warp::reply::html(results));
}

async fn handle_listing_summaries_request(mut req: Vec<TradeUtxoRequest>) -> Result<impl Reply, Rejection> {
    let mut results: Vec<ContractListingResponse> =  Vec::new();
    req.truncate(5000);
    for mut entry in req {
        let import = match get_contract_header(entry.contract_id.clone().as_str()){
            Ok(import) => import,
            Err(_) => {
                results.push(ContractListingResponse::default());
                continue;
            },
        };

        entry.utxos.truncate(5000);
        match get_listing_summaries(&entry.contract_id, entry.utxos.clone(), false) {
            Ok(result) => {
                let response = ContractListingResponse{
                    contract_id: entry.contract_id.clone(),
                    ticker: import.ticker.clone(),
                    rest_url: import.rest_url.clone(),
                    contract_type: import.contract_type.clone(),
                    decimals: import.decimals,
                    listing_summaries: result
                };

                results.push(response);
                continue;
            }
            Err(_) => {}
        }

        match get_listing_summaries(&entry.contract_id, entry.utxos.clone(), true) {
            Ok(result) => {
                let response = ContractListingResponse{
                    contract_id: entry.contract_id.clone(),
                    ticker: import.ticker,
                    rest_url: import.rest_url,
                    contract_type: import.contract_type,
                    decimals: import.decimals,
                    listing_summaries: result
                };
                results.push(response);
            }
            Err(_) => {
   
            }
        }
    }

    Ok(warp::reply::json(&results))
}

async fn handle_listings_for_bids_request(mut req: Vec<TradeUtxoRequest>) -> Result<impl Reply, Rejection> {
    let mut results: Vec<ContractTradeResponse> =  Vec::new();
    req.truncate(5000);
    for mut item in req {
        item.utxos.truncate(5000);
        match get_trade_details_from_bid_utxo(&item.contract_id, item.utxos.clone()) {
            Ok(result) => results.extend(result),
            Err(_) => {
                results.push(ContractTradeResponse::default());
                continue;
            }
        }
    }

    Ok(warp::reply::json(&results))
}

 async fn handle_check_txid_request( mut data :TxidCheck) -> Result<impl Reply, Rejection> {
    let mut results: Vec<TxidCheckResponse> = Vec::new();
    data.contract_ids.truncate(5000);
    for contract_id in data.contract_ids.clone() {
        let mut entry = TxidCheckResponse::default();
        entry.contract_id = contract_id.clone();
        match check_txid_history(&contract_id, &data.txids) {
            Ok(result) => entry.entries.extend( result),
            Err(_) => continue,
        };

        results.push(entry);
    }

     return Ok(warp::reply::json(&results));
 }

// Warp get route functions
async fn handle_get_health() -> Result<impl Reply, Rejection> {
    return Ok(warp::reply::with_status(warp::reply(), warp::http::StatusCode::OK));
}

async fn handle_get_contracts() -> Result<impl Reply, Rejection> {
    return match get_contracts() {
        Ok(contracts) => {
            let result = match serde_json::to_string(&contracts) {
                Ok(result) => result,
                Err(_) => return Err(warp::reject::custom(CustomError {
                    message: "Unable to get contract payloads".to_string(),
                })),
            };
            Ok(warp::reply::html(format!("{}", result)))
        },
        Err(_) => {         
            Ok(warp::reply::html(format!("No Contracts")))
        }
    }  
}

async fn handle_get_contract_field(contract_id: String, field: String)  -> Result<impl Reply, Rejection>{
    let pending:bool;
    let command:String;
    if field.clone().contains("pending-") {
        command = field.replace("pending-","");
        pending = true;
    }else{
        pending = false;
        command = field;
    }

    match get_contract_field(&contract_id, &command, pending, 1){
        Ok(result) =>  return Ok(warp::reply::html(format!("{}", result))),
        Err(error) => {
            let error = CustomError { message: error,};
            return Err(warp::reject::custom(error));
        },
    };
}

async fn handle_get_contract_field_paged(contract_id: String, field: String, page: String)  -> Result<impl Reply, Rejection>{
    let pending:bool;
    let command:String;
    if field.clone().contains("pending-") {
        command = field.replace("pending-","");
        pending = true;
    }else{
        pending = false;
        command = field;
    }

    let mut page_number = match page.parse::<usize>() {
        Ok(page_number) => page_number,
        Err(_) => {
            let error = CustomError { message: "Invalid page number".to_string(),};
            return Err(warp::reject::custom(error));
        }
    };

    if page_number <= 0 {
        page_number = 1;
    }

    match get_contract_field(&contract_id, &command, pending, page_number){
        Ok(result) =>  return Ok(warp::reply::html(format!("{}", result))),
        Err(error) => {
            let error = CustomError { message: error,};
            return Err(warp::reject::custom(error));
        },
    };
}

async fn handle_get_utxo_data(contract_id: String, field: String, utxo:String) -> Result<impl Reply, Rejection>{
    let pending:bool;
    let command:String;
    if field.clone().contains("pending-") {
        command = field.replace("pending-","");
        pending = true;
    }else{
        pending = false;
        command = field;
    }

    match get_utxo_field(&contract_id, &command, utxo, pending){
        Ok(result) =>  return Ok(warp::reply::html(format!("{}", result))),
        Err(error) => {
            let error = CustomError { message: error,};
            return Err(warp::reject::custom(error));
        },
    };
}

async fn handle_get_tx_history(contract_id: String) -> Result<impl Reply, Rejection>{
    let mut entries: Vec<ContractHistoryEntry> = Vec::new();
    let payloads: HashMap<String,String>;
    let contract = match read_contract_scl01(&contract_id, false){
        Ok(contract) => contract,
        Err(_) => return Err(reject::custom(CustomError{ message : "Unable to read contract".to_string()})),
    };

    payloads = contract.payloads;
    for payload in payloads.clone() {
        match extract_info_from_payload(&payload.0, &payload.1, &contract_id, false) {
            Ok(data) => entries.extend(data),
            Err(_) => continue
        };  
    }

    return Ok(warp::reply::json(&entries));
}

async fn handle_custom_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    if let Some(custom_error) = err.find::<CustomError>() {
        // Handle the custom rejection and return a 400 Bad Request response
        let response = warp::reply::with_status(
            warp::reply::html(format!("Bad Request: {}", custom_error)),
            warp::http::StatusCode::BAD_REQUEST,
        );
        Ok(response)
    } else {
        // For other rejections, return a generic 500 Internal Server Error response
        Ok(warp::reply::with_status(
            warp::reply::html("Internal Server Error".to_string()),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

//Server Queue Functions
async fn perform_commands(txid: &str, payload: &str, bid_payloads: &Option<Vec<BidPayload>>, pending: bool, esplora: &String){    
    let commands = match extract_commands(payload){
        Ok(commands) => commands,
        Err(_) => return,  
    };

    for command in commands {
        let contract_id = match extract_contract_id(&command) {
            Ok(contract_id) => contract_id,
            Err(_) => continue,
        };

        if command.contains("SCL01") {
            perform_minting_scl01(txid, &payload);
            return ;
        }else if command.contains("SCL02") {
            perform_minting_scl02(txid, &payload);
            return;
        }else if command.contains("SCL03") {
            perform_minting_scl03(txid, &payload);
            return;
        }else if command.contains("TRANSFER"){
            perform_transfer_scl01(txid, &command, &payload, pending, esplora.to_string()).await;
        }else if command.contains("BURN"){
            perform_burn_scl01(&txid, &command, &payload, pending, esplora.to_string()).await;
        }else if command.contains(":LIST"){
            perform_list_scl01(&txid, &command, &payload, pending, esplora.to_string()).await;
        }else if command.contains(":BID"){
            let payloads = match bid_payloads {
                Some(payloads) => payloads,
                None => continue,
            };

            for bid_payload in payloads{
                if contract_id == bid_payload.contract_id {
                    perform_bid_scl01(&txid, &command, &payload, &bid_payload.trade_txs, pending).await;
                    break;
                }
            }
        }else if command.contains("ACCEPT_BID"){
            perform_accept_bid_scl01(&txid, &command, pending, esplora.to_string()).await;
        }else if command.contains("FULFIL_TRADE"){ 
            perform_fulfil_bid_scl01(&txid, &command, pending).await;
        }else if payload.contains("CANCELLISTING"){
            perform_listing_cancel_scl01(txid, &payload, pending, esplora.to_string()).await;
        }else if payload.contains("CANCELBID"){
            perform_bid_cancel_scl01(txid, &payload, pending, esplora.to_string()).await;
        }else if command.contains("DRIP"){
            perform_drip_start_scl01(txid,&command, &payload, pending, esplora.to_string()).await;
        }else if command.contains(":DIMAIRDROP"){
            perform_create_diminishing_airdrop_scl01(txid,&command, &payload, pending, esplora.to_string()).await;
        }else if command.contains("CLAIM_DIMAIRDROP"){
            perform_claim_diminishing_airdrop_scl01(txid,&command, &payload, pending, esplora.to_string()).await;
        }else if command.contains(":DGE"){
            perform_create_dge_scl01(txid,&command, &payload, pending, esplora.to_string()).await;
        }else if command.contains("CLAIM_DGE"){
            perform_claim_dge_scl01(txid, &command, &payload, pending, esplora.to_string()).await;
        }else if command.contains("AIRDROP"){
            perform_airdrop(txid, &command, &payload, pending);
        }else if command.contains("RIGHTTOMINT"){
            perform_rights_to_mint(txid,&command, &payload, pending, esplora.to_string()).await;
        }
    }
}

async fn great_sort(){
    let mut pending_queue: Vec<(PendingCommandStruct, String)> = match read_queue(PENDINGCOMMANDSPATH.to_string()){
        Ok(pending_queue) => pending_queue,
        Err(_) => Vec::new(),
    };

    let claim_queue: Vec<(PendingCommandStruct, String)> = match read_queue("./Json/Queues/Claims/".to_string()){
        Ok(queue) => queue,
        Err(_) => Vec::new(),
    };

    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let esplora = match config.esplora{
        Some(esplora) => esplora,
        None => "https://btc.darkfusion.tech/".to_owned(),
    };

    _ = perform_contracts_checks().await;

    if claim_queue.len() > 0 {
        let mut claims: Vec<(PendingCommandStruct, String, bool, bool, u64)> = Vec::new();
        for command in claim_queue{
            let res = payload_validation_and_confirmation(command.0.txid.as_str(), command.0.payload.as_str()).await;
            let command_date = match NaiveDateTime::parse_from_str(&command.0.time_added, "%Y-%m-%d %H:%M:%S")  {
                Ok(command_date) => command_date,
                Err(_) => continue,
            };
    
            let duration = Local::now().naive_local() - command_date;
            let two_mins = chrono::Duration::minutes(2);
            if res.0 == false {
                if duration > two_mins {
                    let path_from_string: &Path = Path::new(&command.1);
                    if path_from_string.is_file() {
                         let _res = fs::remove_file(&path_from_string);
                    }
                }
    
                continue;
            }
    
            claims.push((command.0, command.1, res.0 ,res.1, res.2));
        }
    
        claims.sort_by(|(_, _,_, _, amount_1), (_, _,_, _, amount_2)| amount_2.cmp(amount_1));
    
        for command in claims{
            let command_date = match NaiveDateTime::parse_from_str(&command.0.time_added, "%Y-%m-%d %H:%M:%S")  {
                Ok(command_date) => command_date,
                Err(_) => continue,
            };
    
            let duration = Local::now().naive_local() - command_date;
            let two_mins = chrono::Duration::minutes(2);
            if command.2 == false {
                if duration > two_mins {
                    let path_from_string: &Path = Path::new(&command.1);
                    if path_from_string.is_file() {
                         let _res = fs::remove_file(&path_from_string);
                    }
                }
    
                continue;
            }
    
            if !command.3 {
                let twenty_four_hours = chrono::Duration::hours(24);
                if duration >= twenty_four_hours {
                    continue;
                }
                
                perform_commands(command.0.txid.as_str(), command.0.payload.as_str(), &command.0.bid_payload, true, &esplora).await;
            }else{
                perform_commands(command.0.txid.as_str(), command.0.payload.as_str(), &command.0.bid_payload, false, &esplora).await;
                let path_from_string: &Path = Path::new(&command.1);
                if path_from_string.is_file() {
                     let _res = fs::remove_file(&path_from_string);
                }
            }
        }
    }

    if pending_queue.len() == 0 {
        return ;
    }

    pending_queue.sort_by(|(_, string_a), (_, string_b)| string_a.cmp(string_b));

    for command in pending_queue{
        let res = payload_validation_and_confirmation(command.0.txid.as_str(), command.0.payload.as_str()).await;
        let command_date = match NaiveDateTime::parse_from_str(&command.0.time_added, "%Y-%m-%d %H:%M:%S")  {
            Ok(command_date) => command_date,
            Err(_) => continue,
        };

        let duration = Local::now().naive_local() - command_date;
        let two_mins = chrono::Duration::minutes(2);
        if res.0 == false {
            if duration > two_mins {
                let path_from_string: &Path = Path::new(&command.1);
                if path_from_string.is_file() {
                     let _res = fs::remove_file(&path_from_string);
                }
            }

            continue;
        }

        if !res.1 {
            let twenty_four_hours = chrono::Duration::hours(24);
            if duration >= twenty_four_hours {
                continue;
            }
            
            perform_commands(command.0.txid.as_str(), command.0.payload.as_str(), &command.0.bid_payload, true, &esplora).await;
        }else{
            perform_commands(command.0.txid.as_str(), command.0.payload.as_str(), &command.0.bid_payload, false, &esplora).await;
            let path_from_string: &Path = Path::new(&command.1);
            if path_from_string.is_file() {
                 let _res = fs::remove_file(&path_from_string);
            }
        }
    }
}

// Validation
async fn payload_validation_and_confirmation(txid: &str, payload: &str) -> (bool, bool, u64) { 
    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let esplora = match config.esplora{
        Some(esplora) => esplora,
        None => "https://btc.darkfusion.tech/".to_owned(),
    };

    let url = format!("{}tx/{}",esplora, txid);
    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => return (false, false, 0),
    };

    let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
        Ok(tx_info) => tx_info,
        Err(_) => return (false, false, 0),
    };

    let trim1 = trim_chars(&payload, "\r");
    let trim2 = trim_chars(&trim1, "\n");
    let payload_hash = hex_digest(Algorithm::SHA256, &trim2.as_bytes());
    let vout = match tx_info.vout {
        Some(vout) => vout,
        None => return (false, false, 0),
    };


    for output in vout {
        let scriptpubkey_type = match output.scriptpubkey_type {
            Some(scriptpubkey_type) => scriptpubkey_type,
            None => return (false, false, 0),
        };

        if scriptpubkey_type == "op_return".to_string() {
            let scriptpubkey_asm = match output.scriptpubkey_asm {
                Some(scriptpubkey_asm) => scriptpubkey_asm,
                None => return (false, false, 0),
            };

            let mut hash_check: String = scriptpubkey_asm;
            let hash_check_index = match hash_check.find("OP_PUSHBYTES_32") {
                Some(hash_check_index) => hash_check_index,
                None => return (false, false, 0),
            };

            hash_check = hash_check[hash_check_index + 16..].to_string();
            if payload_hash == hash_check {
                let status = match tx_info.status {
                    Some(status) => status,
                    None => return (false, false, 0),
                };

                let confirmed = match status.confirmed {
                    Some(confirmed) => confirmed,
                    None => return (false, false, 0),
                };

                let fee = match tx_info.fee {
                    Some(fee) => fee,
                    None => 0,
                };

                if !confirmed {
                    return (true, false, fee);
                } else {
                    return (true, true, fee);
                }
            };
        }
    }
    return (false, false, 0);
}

async fn perform_contracts_checks() -> Result<String, String> {
    let entries = match fs::read_dir(CONTRACTSPATH.to_string()){
        Ok(entries) => entries,
        Err(_) => return Ok("Success".to_string()),
    };

    let current_block = match get_current_block_height().await{
        Ok(current_block) => current_block,
        Err(_) => return Err("Unable to get contract type".to_string())
    };

    let config = match read_server_config(){
        Ok(config) => config,
        Err(_) => return Err("Unable to server config".to_string()),
    };

    if config.block_height < current_block {
        let c = Config{
            block_height: current_block,
            memes: config.memes,
            sorting: config.sorting,
            sort_again: config.sort_again,
            reserved_tickers: config.reserved_tickers,
            hosts_ips: config.hosts_ips,
            my_ip_split: config.my_ip_split,
            my_ip: config.my_ip,
            key: config.key,
            esplora: config.esplora,
            url: config.url
        };

        let _ = save_server_config(c);
    }

    for entry in entries {
        // Reset pending contracts
        if let Ok(entry) = entry {
            let pending_path = format!("{}/pending.txt", entry.path().to_string_lossy());
            let path = format!("{}/state.txt", entry.path().to_string_lossy());
            if !fs::metadata(&pending_path).is_ok() || !fs::metadata(&path).is_ok()  {
                continue;
            }

            let state_str = match read_from_file(path) {
                Some(state_str) => state_str,
                None => continue,
            };

            write_to_file(pending_path, state_str);
            let directory_name = entry
                .path()
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .map(|s| s.to_string());
            if config.block_height < current_block {
                let contract_id = match directory_name {
                    Some(contract_id) => contract_id,
                    None => continue,
                };

		        _ = perform_drips_scl01(contract_id.clone(), current_block as u64, false);
                let contract = match read_contract_scl01(contract_id.as_str(), false) {
                    Ok(contract) => contract,
                    Err(_) => return Err("Failed to find contract".to_string())
                };

                if let Some(bids) = contract.bids.clone() {
                    for (key, value) in bids  {
                        match add_fulfillment_commands_to_queue(&value.accept_tx, &key, &contract_id).await{
                            Ok(_) => {},
                            Err(_) =>  continue, 
                        };
                    }
                }

                if let Some(_split) = contract.clone().last_airdrop_split {
                    perform_airdrop_split(contract)
                }
            }
        }
    }

    return Ok("Success".to_string());
}

fn get_contracts() -> Result<Vec<String>, String> {
    // Attempt to read the entries in the folder
    let entries = match fs::read_dir(CONTRACTSPATH.to_string()){
        Ok(entries) => entries,
        Err(_) => return Err("Error read".to_string())
    };

    // Filter entries to only include files and extract file names
    let file_names: Vec<String> = entries
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                if e.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    Some(e.file_name())
                } else {
                    None
                }
            })
        })
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .collect();

    Ok(file_names)
}

fn get_contract_field(contract_id: &String, field: &String, pending: bool, mut page: usize) -> Result<String,String>{
    let contract = match read_contract_scl01(contract_id, pending){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to read contract".to_string()),
    };

    match field.as_str() {
        "state" => {
            let result = match serde_json::to_string(&contract){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to read contract".to_string()), 
            };
            return Ok(format!("{}", result));
        }
        
        "ticker" => {
            return Ok(format!("{{\"Ticker\":\"{}\"}}", contract.ticker.to_string()));
        }
        
        "contractid" => {
            return Ok(format!("{{\"ContractID\":\"{}\"}}", contract.contractid.to_string()));
        }
        
        "supply" => {
            return Ok(format!("{{\"Supply\":\"{}\"}}", contract.supply.to_string()));
        }
        
        "owners" => {
            let total_pages = contract.owners.len()/ 100;
            if contract.owners.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = contract.owners.clone().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));       
                 let owners: Vec<_> = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                 let result = match serde_json::to_string(&owners){
                    Ok(result) =>  result,
                    Err(_) => return Err("Unable to get contract owners".to_string()), 
                };

                
                let data = contruct_pagination_metadata(result, page, total_pages, 100);
                return Ok(format!("{}", data));
            }else{
                let owners_vec:Vec<_> = contract.owners.into_iter().collect();
                let result = match serde_json::to_string(&owners_vec){
                    Ok(result) =>  result,
                    Err(_) => return Err("Unable to get contract owners".to_string()), 
                };

                let data = contruct_pagination_metadata(result, page, total_pages, 100);
                return Ok(format!("{}", data));
            }
        }
        
        "payloads" => {
            let total_pages = contract.payloads.len()/ 100;
            if contract.payloads.len() > 100 {
                let mut sorted_entries: Vec<_> = contract.payloads.clone().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));       
                let payloads: Vec<_> = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&payloads){
                   Ok(result) =>  result,
                   Err(_) => return Err("Unable to get contract payloads".to_string()), 
               };

               if page > total_pages {
                  page = total_pages;
               }

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let payloads_vec:Vec<_> = contract.payloads.into_iter().collect();
            let result = match serde_json::to_string(&payloads_vec){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to get contract payloads".to_string()), 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "decimals" => {
            return Ok(format!("{{\"Decimals\":\"{}\"}}", contract.decimals.to_string()));
        }
        
        "listings" => {
            let listings = match &contract.listings{
                Some(lisitngs) => lisitngs,
                None => return Ok("{}".to_string()), 
            };

            let total_pages = listings.len()/ 100;
            if listings.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = listings.into_iter().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));       
                let filtered_listings: Vec<_>  = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&filtered_listings){
                   Ok(result) =>  result,
                   Err(_) => return Ok("{}".to_string()), 
               };

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let listings_vec:Vec<_> = listings.into_iter().collect();
            let result = match serde_json::to_string(&listings_vec){
                Ok(result) =>  result,
                Err(_) => return Ok("{}".to_string()), 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "bids" => {
            let bids = match &contract.bids{
                Some(bids) => bids,
                None => return Ok("{}".to_string()), 
            };

            let total_pages = bids.len()/ 100;
            if bids.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = bids.to_owned().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));
                let filtered_bids: Vec<_>  = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&filtered_bids){
                   Ok(result) =>  result,
                   Err(_) => return Ok("{}".to_string()), 
               };

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let bids_vec:Vec<_> = bids.into_iter().collect();
            let result = match serde_json::to_string(&bids_vec){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to get contract pending bids".to_string()) 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "fulfillments" => {
            let fulfillments = match &contract.fulfillments{
                Some(fulfillments) => fulfillments,
                None => return Ok("{}".to_string()), 
            };

            let total_pages = fulfillments.len()/ 100;
            if fulfillments.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = fulfillments.to_owned().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));
                let filtered_fulfillments: Vec<_>  = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&filtered_fulfillments){
                   Ok(result) =>  result,
                   Err(_) => return Ok("{}".to_string()), 
               };

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let fulfillments_vec:Vec<_> = fulfillments.into_iter().collect();
            let result = match serde_json::to_string(&fulfillments_vec){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to get contract pending fulfillments".to_string()) 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "airdrop_amount" => {
            let airdrop_amount = match contract.airdrop_amount {
                Some(airdrop_amount) => airdrop_amount,
                None => 0,
            };

            return Ok(format!("{{\"Airdrop_Amount\":\"{}\"}}", airdrop_amount.to_string()));
        }
        
        "dges" => {
            let dges = match &contract.dges{
                Some(dges) => dges,
                None => return Ok("{}".to_string()), 
            };

            let total_pages = dges.len()/ 100;
            if dges.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = dges.to_owned().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));
                let filtered_dges: Vec<_>  = dges.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&filtered_dges){
                   Ok(result) =>  result,
                   Err(_) => return Ok("{}".to_string()), 
               };

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let dges_vec:Vec<_> = dges.into_iter().collect();
            let result = match serde_json::to_string(&dges_vec){
                Ok(result) =>  result,
                Err(_) => return Ok("{}".to_string()), 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "dim_airdrop" => {
            let diminishing_airdrops = match &contract.diminishing_airdrops{
                Some(diminishing_airdrops) => diminishing_airdrops,
                None => return Ok("{}".to_string()), 
            };

            let total_pages = diminishing_airdrops.len()/ 100;
            if diminishing_airdrops.len() > 100 {
                if page > total_pages {
                    page = total_pages;
                }

                let mut sorted_entries: Vec<_> = diminishing_airdrops.to_owned().drain().collect();
                sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));
                let filtered_diminishing_airdrops: Vec<_>  = sorted_entries.iter().skip(100 * (page - 1)).take(100).collect();
                let result = match serde_json::to_string(&filtered_diminishing_airdrops){
                   Ok(result) =>  result,
                   Err(_) => return Ok("{}".to_string()), 
               };

               let data = contruct_pagination_metadata(result, page, total_pages, 100);
               return Ok(format!("{}", data));
           }else{
            let diminishing_airdrops_vec:Vec<_> = diminishing_airdrops.into_iter().collect();
            let result = match serde_json::to_string(&diminishing_airdrops_vec){
                Ok(result) =>  result,
                Err(_) => return Ok("{}".to_string()), 
            };

            let data = contruct_pagination_metadata(result, page, total_pages, 100);
            return Ok(format!("{}", data));
           }
        }
        
        "current_amount_airdropped" => {
            let airdrop_amount = match contract.airdrop_amount {
                Some(airdrop_amount) => airdrop_amount,
                None => 0,
            };

            let current_airdrops = match contract.current_airdrops {
                Some(current_airdrops) => current_airdrops,
                None => 0,
            };

            return Ok(format!("{{\"Current_Airdrop_Amount\":\"{}\"}}", (current_airdrops * airdrop_amount).to_string()));
        }
        
        "import_contract" => {
            let import = match get_contract_header(contract.contractid.clone().as_str()){
                Ok(import) => import,
                Err(_) => return Err("Unable to get contract header".to_string()),
            };

            let result = match serde_json::to_string(&import){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to get contract header".to_string()), 
            };
            return Ok(format!("{}", result));
        }
        
        "summary" => {
            let summary = match get_scl01_contract_summary(&contract){
                Ok(summary) => summary,
                Err(_) => return Err("Unable to get contract summary".to_string()),
            };
            return Ok(format!("{}", summary));
        }
        
        "trades" => {
            let interactions =  match read_contract_interactions(&contract.contractid) {
                Ok(interactions) => interactions,
                Err(_) => return Err("Unable to get contract fulfillments".to_string())
            };
            let result = match serde_json::to_string(&interactions.fulfillment_summaries){
                Ok(result) =>  result,
                Err(_) => return Err("Unable to get contract fulfillments".to_string()), 
            };

            return Ok(format!("{}", result));
        }
        
        s if s.contains(":") => {
            match contract.owners.get(s) {
                 Some(result) => return Ok(result.to_string()),
                 None => return Ok( "{\"Result\":\"UTXO specified is unbound.\"}".to_string())
            };
        }

         _ => {
            return Err("Unknown contract endpoint".to_string());
        }
    }
}

fn get_utxo_field(contract_id: &String, field: &String, utxo: String, pending: bool) -> Result<String,String>{
    let contract = match read_contract_scl01(contract_id, pending){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to get contract".to_string()),
    };

    match field.as_str() {
        "bids_on_listing" => {
            let bids = match &contract.bids{
                Some(bids) => bids,
                None => return Ok("{}".to_string()), 
            };

            let listings = match &contract.listings{
                Some(listings) => listings,
                None => return Ok("{}".to_string()), 
            };

            let filter_listing: Vec<(&String, &scl01_contract::Listing)> = listings
            .into_iter()
            .filter(|(_, listing)| listing.list_utxo == utxo.to_string())
            .collect();

            if filter_listing.len() != 1 {
                return Ok("{}".to_string());
            }
             
            let listing_bids: HashMap<&String, &scl01_contract::Bid> = bids
            .into_iter()
            .filter(|(_, bid)| bid.order_id == filter_listing[0].0.to_string())
            .collect();

            let result_string = match serde_json::to_string(&listing_bids){
                Ok(result_string) =>  result_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            return Ok(format!("{}", result_string));
        }

        "bids_summary_on_listing" => {
            let listings = match &contract.listings{
                Some(lisitngs) => lisitngs,
                None => return Ok("{}".to_string()), 
            };

            let filter_listing: HashMap<&String, &scl01_contract::Listing> = listings
            .into_iter()
            .filter(|(_, listing)| listing.list_utxo == utxo.to_string())
            .collect();

            if filter_listing.len() != 1 {
                return Ok("{}".to_string());
            }

            let mut listing = scl01_contract::Listing::default();
            let mut key = "".to_string();
            if let Some((k, value)) = filter_listing.iter().next() {
                listing = value.to_owned().clone();
                key = k.to_string();
            }

            let mut result = "{\"listing\" :".to_string();
            let listing_string = match serde_json::to_string(&listing){
                Ok(listing_string) =>  listing_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            result.push_str(&listing_string);
            result.push_str(",\"bids\" :");
            let bids: HashMap<String, scl01_contract::Bid> = match contract.bids{
                Some(bids) => bids,
                None => {
                    result.push_str("{}");
                    result.push_str("}");
                    return Ok(format!("{}", result));
                }, 
            };
             
            let listing_bids: HashMap<String, scl01_contract::Bid> = bids
            .into_iter()
            .filter(|(_, bid)| bid.order_id == key)
            .collect();

            let mut bid_data: HashMap<String, BidData> = HashMap::new();
            for (key, bid) in listing_bids {
                let data = BidData{ bid_price: bid.bid_price.to_string(), bid_amount: bid.bid_amount.to_string(), order_id: bid.order_id, fulfill_tx: bid.fulfill_tx, accept_tx: bid.accept_tx, reseved_utxo: bid.reseved_utxo };
                bid_data.insert(key, data);
            }

            let result_string = match serde_json::to_string(&bid_data){
                Ok(result_string) =>  result_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            result.push_str(&result_string);
            result.push_str("}");
            return Ok(format!("{}", result))
        }

        "listing_for_bid" => {
            let bids = match &contract.bids{
                Some(bids) => bids,
                None => return Ok("{}".to_string()), 
            };

            let listings = match &contract.listings{
                Some(listings) => listings,
                None => return Ok("{}".to_string()), 
            };
            
            let listing_bids: Vec<(&String, &scl01_contract::Bid)> = bids
            .into_iter()
            .filter(|(_, bid)| bid.reseved_utxo == utxo.to_string())
            .collect();

            if listing_bids.len() != 1 {
                return Ok("{}".to_string());
            }

            let listing = match listings.get(&listing_bids[0].1.order_id){
                Some(listing) => listing,
                None => return Ok("{}".to_string()),
            };

            let result_string = match serde_json::to_string(&listing){
                Ok(result_string) =>  result_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            return Ok(format!("{}", result_string));
        }

        "listing" => {
            let listings = match &contract.listings{
                Some(listings) => listings,
                None => return Ok("{}".to_string()), 
            };

            let filter_listing: Vec<(&String, &scl01_contract::Listing)> = listings
            .into_iter()
            .filter(|(_, listing)| listing.list_utxo == utxo.to_string())
            .collect();

            if filter_listing.len() != 1 {
                return Ok("{}".to_string());
            }

            let result_string = match serde_json::to_string(&filter_listing[0]){
                Ok(result_string) =>  result_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            return Ok(format!("{}", result_string));
        }

        "bid" => {
            let bids = match &contract.bids{
                Some(bids) => bids,
                None => return Ok("{}".to_string()), 
            };

            let listing_bids: Vec<(&String, &scl01_contract::Bid)> = bids
            .into_iter()
            .filter(|(_, bid)| bid.reseved_utxo == utxo.to_string())
            .collect();

            if listing_bids.len() != 1 {
                return Ok("{}".to_string());
            }

            let result_string = match serde_json::to_string(&listing_bids[0].1){
                Ok(result_string) =>  result_string,
                Err(_) => return Ok("{}".to_string()), 
            };

            return Ok(format!("{}", result_string));
        }

        "fulfillment" => {
            let fulfillments = match &contract.fulfillments{
                Some(fulfillments) => fulfillments,
                None => return Err( "Unable to get contract fulfillments".to_string()), 
            };

            let result = match fulfillments.get(&utxo) {
                Some(result) => result,
                None => return Ok("{\"Result\":\"UTXO specified is unbound.\"}".to_string())
            };

            let result_string = match serde_json::to_string(&result){
                Ok(result_string) =>  result_string,
                Err(_) => return Err("Unable to get contract fulfillment".to_string()), 
            };

            return Ok(format!("{}", result_string));
        }

        "owner" => {
            let _result = match contract.owners.get(&utxo) {
                  Some(result) => return Ok(format!("{}", result.to_string())),
                  None => return Ok("{\"Result\":\"UTXO specified is unbound.\"}".to_string())
             };
         }
         _ => { return Err("Unknown contract endpoint".to_string());}
    }
}

fn get_listing_summaries(contract_id: &String, listing_utxos: Vec<String>, pending: bool)-> Result<Vec<ListingSummary>,String>{
    let mut summaries: Vec<ListingSummary> = Vec::new();
        let contract = match read_contract_scl01(contract_id, pending){
            Ok(contract) => contract,
            Err(_) => return Err("Unable to read contract".to_string()),
        };

        let listings = match contract.listings{
            Some(listings) => listings,
            None => return Err("Unable to get listings".to_string()),
        };

        let bids = match contract.bids{
            Some(bids) => bids,
            None => HashMap::new(),
        };

        for listing_utxo in listing_utxos {
            let mut summary =  ListingSummary::default();
            let mut order_id = "".to_string();
            for (key, value) in &listings {
                if value.list_utxo == listing_utxo{
                    summary.list_price = value.price;
                    summary.quantity = value.list_amt;
                    summary.listing_utxo = listing_utxo.clone();
                    order_id =  key.to_string();
                    summary.pending_listing = pending;
                    break;
                }
            }
            
            let listing_bids: HashMap<String, scl01_contract::Bid> = bids.clone()
            .into_iter()
            .filter(|(_, bid)| bid.order_id == order_id)
            .collect();

            summary.bid_count = listing_bids.len() as u64;
            if let Some(highest) = listing_bids.iter().max_by_key(|&(_, value)| value.bid_price){
            summary.highest_bid = highest.1.bid_price;
            }

            summaries.push(summary);
        }

    return Ok (summaries);
}

fn get_trade_details_from_bid_utxo(contract_id: &String, bid_utxos: Vec<String>)-> Result<Vec<ContractTradeResponse> ,String>{
    let mut summaries: Vec<ContractTradeResponse> = Vec::new();
    let mut contract = match read_contract_scl01(contract_id, false){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to read contract".to_string()),
    };

    let mut listings = match contract.listings{
        Some(listings) => listings,
        None => return Err("Unable to get listings".to_string()),
    };

    let mut bids = match contract.bids{
        Some(bids) => bids,
        None => HashMap::new(),
    };

    let mut pending_check: Vec<String> = Vec::new();
    for bid_utxo in bid_utxos {
        let mut summary: ContractTradeResponse = ContractTradeResponse::default();
        summary.contract_id = contract_id.to_string();
        let listing_bids: Vec<(String, scl01_contract::Bid)> = bids.clone()
        .into_iter()
        .filter(|(_, bid)| bid.reseved_utxo == bid_utxo.to_string())
        .collect();

        if listing_bids.len() == 1 {
            summary.bid_price = listing_bids[0].1.bid_price;
            summary.bid_amount = listing_bids[0].1.bid_amount;
            summary.order_id = listing_bids[0].1.order_id.clone();
            summary.bid_utxo = bid_utxo.clone();
            summary.bid_pending = false;
            
            let listing = match listings.get(&summary.order_id) {
                Some(result) => result,
                None => {
                    pending_check.push(bid_utxo.clone());
                    continue;
                },
            };
    
            summary.listing_amount = listing.list_amt;
            summary.listing_price = listing.price;
            summary.listing_utxo = listing.list_utxo.to_string();
            summaries.push(summary);
        }else{
            pending_check.push(bid_utxo.clone());
        }
    }

    if pending_check.len() == 0  {
        return Ok (summaries);
    }

    contract = match read_contract_scl01(contract_id, true){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to read contract".to_string()),
    };

    listings = match contract.listings{
        Some(listings) => listings,
        None => return Err("Unable to get listings".to_string()),
    };

    bids = match contract.bids{
        Some(bids) => bids,
        None => HashMap::new(),
    };

    for bid_utxo in pending_check {
        let mut summary: ContractTradeResponse = ContractTradeResponse::default();
        summary.contract_id = contract_id.to_string();
        let listing_bids: Vec<(String, scl01_contract::Bid)> = bids.clone()
        .into_iter()
        .filter(|(_, bid)| bid.reseved_utxo == bid_utxo.to_string())
        .collect();

        if listing_bids.len() == 1 {
            summary.bid_price = listing_bids[0].1.bid_price;
            summary.bid_amount = listing_bids[0].1.bid_amount;
            summary.order_id = listing_bids[0].1.order_id.clone();
            summary.bid_utxo = bid_utxo.clone();
            summary.bid_pending = true;
            
            let listing = match listings.get(&summary.order_id) {
                Some(result) => result,
                None => {
                    let mut response = ContractTradeResponse::default();
                    response.bid_utxo = bid_utxo.clone();
                    response.contract_id = contract_id.clone();
                    summaries.push(response);
                    continue;
                },
            };
    
            summary.listing_amount = listing.list_amt;
            summary.listing_price = listing.price;
            summary.listing_utxo = listing.list_utxo.to_string();
            summaries.push(summary);
        }else{
            let mut response = ContractTradeResponse::default();
            response.bid_utxo = bid_utxo.clone();
            response.contract_id = contract_id.clone();
            summaries.push(response);
            continue;
        }
    }

    return Ok (summaries);
}

fn check_txid_history(contract_id: &String, txids: &Vec<String>) -> Result<Vec<ContractHistoryEntry>,String>{
    let mut entries: Vec<ContractHistoryEntry> = Vec::new();
    let payloads: HashMap<String,String>;
    let pending_payloads: HashMap<String,String>;
    let contract = match read_contract_scl01(contract_id, false){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to read contract".to_string()),
    };

    let pending_contract = match read_contract_scl01(contract_id, true){
        Ok(contract) => contract,
        Err(_) => return Err("Unable to read contract".to_string()),
    };

    payloads = contract.payloads;
    pending_payloads = pending_contract.payloads;

    for (payload_txid, payload) in payloads.clone() {
        if txids.contains(&payload_txid) {
            match extract_info_from_payload(&payload_txid, &payload, &contract_id, false) {
                Ok(data) => entries.extend(data),
                Err(_) => continue
            };  
        }
    }

    let dif_payloads: HashMap<_, _> = pending_payloads
        .iter()
        .filter(|(key, _)| !payloads.contains_key(*key))
        .collect();

    for (payload_txid, payload) in dif_payloads {
        match extract_info_from_payload(payload_txid, payload, &contract_id, true) {
            Ok(data) => entries.extend(data),
            Err(_) => continue,
        };
    }

    return Ok(entries);
}

fn extract_info_from_payload(txid: &String, payload: &String, contract_id: &String, pending: bool) -> Result<Vec<ContractHistoryEntry>,String>{
    let mut commands = match extract_commands(payload){
        Ok(commands) => commands,
        Err(err) => return Err(err),  
    };
    
    if commands.len() == 0 {
        commands.push(payload.to_string());
    }

    let mut entries: Vec<ContractHistoryEntry> = Vec::new();
    for command in commands {
        let current_contract_id = match extract_contract_id(payload) {
            Ok(contract_id) => contract_id,
            Err(_) => continue,
        };

        if contract_id.clone() !=  current_contract_id && !command.contains("SCL"){
            continue;
        }

        if command.contains("AIRDROP"){
            let contract = match read_contract_scl01(contract_id, false){
                Ok(contract) => contract,
                Err(_) => continue,
            };

            let airdrop_amount = match contract.airdrop_amount{
                Some(airdrop_amount) => airdrop_amount,
                None => continue,
            };

            entries.push(ContractHistoryEntry { tx_type: "Airdrop".to_owned(), scl_value: airdrop_amount, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("TRANSFER"){
            let result: (Vec<String>, Vec<(String, u64)>, String) = match scl01_utils::handle_transfer_payload(&txid, &command) {
                Ok(res) => res,
                Err(_) => (Vec::new(), Vec::new(), String::new()),
            };

            let mut total_transfer = 0;
            for (_, value) in result.1 {
                total_transfer += value;
            }

            entries.push(ContractHistoryEntry { tx_type: "Transfer".to_owned(), scl_value: total_transfer, txid: txid.to_owned(), btc_price: None, pending });
        }else if command.contains("BURN"){
            let result = match scl01_utils::handle_burn_payload(&txid, &command) {
                Ok(res) => res,
                Err(_) => (Vec::new(), 0, "".to_string()),
            };

            entries.push(ContractHistoryEntry { tx_type: "Burn".to_owned(), scl_value: result.1, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("SCL01"){
            let result = match scl01_utils::handle_mint_payload(&command, &txid) {
                Ok(res) => res,
                Err(_) => continue,
            };

            entries.push(ContractHistoryEntry { tx_type: "Mint".to_owned(), scl_value: result.2, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("SCL02"){
            let result = match scl01_utils::handle_mint_payload(&command, &txid) {
                Ok(res) => res,
                Err(_) => continue,
            };

            entries.push(ContractHistoryEntry { tx_type: "Mint".to_owned(), scl_value: result.2, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("SCL03"){
            let result = match scl01_utils::handle_mint_rtm_payload(&command, &txid) {
                Ok(res) => res,
                Err(_) => continue,
            };

            let mut max_supply = 0;
            for (_, value) in result.2.clone() {
                max_supply += value;
            }

            entries.push(ContractHistoryEntry { tx_type: "Mint".to_owned(), scl_value: max_supply, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains(":DIMAIRDROP"){
            let result = match scl01_utils::handle_create_diminishing_airdrop_payload(&command, &txid) {
                Ok(res) => res,
                Err(_) => continue,
            };

            entries.push(ContractHistoryEntry { tx_type: "Create Diminishing Airdrop".to_owned(), scl_value: result.1, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("CLAIM_DIMAIRDROP"){
            entries.push(ContractHistoryEntry { tx_type: "Claim Diminishing Airdrop".to_owned(), scl_value: 0, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains(":DGE"){
            let result = match scl01_utils::handle_create_dge_payload(&command, &txid) {
                Ok(res) => res,
                Err(_) => continue,
            };

            entries.push(ContractHistoryEntry { tx_type: "Create DGE".to_owned(), scl_value: result.1, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("CLAIM_DGE"){
            entries.push(ContractHistoryEntry { tx_type: "Claim DGE".to_owned(), scl_value: 0, btc_price: None, txid: txid.to_owned(), pending});
        }else if command.contains("LIST"){
            let result: (Vec<String>, String, String, String, u64, u64) = match scl01_utils::handle_list_payload(&txid, &command) {
                Ok(res) => res,
                Err(_) => (Vec::new(), "".to_string(), "".to_string(), "".to_string(), 0, 0),
            };

            entries.push(ContractHistoryEntry { tx_type: "List".to_owned(), scl_value: result.4, btc_price: Some(result.5), txid: txid.to_owned(), pending});
        }else if command.contains(":BID"){
            let results = match scl01_utils::handle_bid_payload(&txid, &command) {
                Ok(res) => res,
                Err(_) => Vec::new(),
            };

            for res in results {
                entries.push(ContractHistoryEntry { tx_type: "Bid".to_owned(), scl_value: res.1, btc_price: Some(res.2), txid: txid.to_owned(), pending});
            }
        }else if command.contains("ACCEPT_BID"){
            let result = match handle_payload_extra_trade_info(&payload){
                Ok(result) => result,
                Err(_) => {
                    entries.push(ContractHistoryEntry { tx_type: "Accept Bid".to_owned(), scl_value: 0, btc_price: None, txid: txid.to_owned(), pending});
                    continue;
                },
            };
            entries.push(ContractHistoryEntry { tx_type: "Accept Bid".to_owned(), scl_value: result.1, btc_price: Some(result.2), txid: txid.to_owned(), pending});
        }else if command.contains("FULFIL_TRADE"){
            let result = match handle_payload_extra_trade_info(&payload){
                Ok(result) => result,
                Err(_) => {
                    entries.push(ContractHistoryEntry { tx_type: "Fulfil Trade".to_owned(), scl_value: 0, btc_price: None, txid: txid.to_owned(), pending});
                    continue;
                },
            };
            entries.push(ContractHistoryEntry { tx_type: "Fulfil Trade".to_owned(), scl_value: result.1, btc_price: Some(result.2), txid: txid.to_owned(), pending});
        }
    }
    return Ok(entries);
}

async fn add_fulfillment_commands_to_queue(accept_tx: &String, fulfillment_txid: &String, contract_id: &String) -> Result<String,String>{
    let txid = match get_txid_from_hash(&accept_tx){
        Ok(txid) => txid,
        Err(_) => return Err("Get the bid accept txid from the bid accept tx".to_string()), 
    };

    let accept_res = match check_txid_confirmed(&txid).await{
        Ok(res) => res,
        Err(_) => return Err("Failed to check bid accept txid".to_string())
    };
    let accept_payload = format!("{{{}:ACCEPT_BID}}",contract_id);
    _ = add_command_to_queue(&txid, &accept_payload,!accept_res);

    let fulfill_res = match check_txid_confirmed(&fulfillment_txid).await{
        Ok(res) => res,
        Err(_) => return Err("Failed to check fulfilment txid".to_string()),
    };

    let fulfill_payload = format!("{{{}:FULFIL_TRADE}}",contract_id);
    _ = add_command_to_queue(&fulfillment_txid, &fulfill_payload,!fulfill_res);
    return Ok("Added fulfillment commands to queue".to_string());
}

fn add_command_to_queue(txid: &String, payload: &String, pending: bool) -> Result<String, String> {
    if pending{
        let current_date_time = Local::now();
        let formatted_date_time = current_date_time.format("%Y-%m-%d %H:%M:%S").to_string();
        let pending_command = PendingCommandStruct{
            txid: txid.clone() ,
            payload: payload.clone(),
            bid_payload: None,
            time_added: formatted_date_time
        };
        
        let command_str = match serde_json::to_string(&pending_command) {
            Ok(command_str) => command_str,
            Err(_) => return Err("Unable to serialize command data".to_string())
        };
        
        let _res = match enqueue_item(format!("{}{}-{}.txt", PENDINGCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), txid), &command_str.to_string()) {
            Ok(res) => res,
            Err(_) => return Err("Unable to add pending command to queue".to_string()),
        };
    }else{
        let command = CommandStruct{
            txid: txid.clone() ,
            payload: payload.clone(),
            bid_payload: None,
        };

        let command_str = match serde_json::to_string(&command) {
            Ok(command_str) => command_str,
            Err(_) => return Err("Unable to serialize command data".to_string())
        };    

        let _res = match enqueue_item(format!("{}{}-{}.txt", TXCOMMANDSPATH, Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(), txid), &command_str.to_string()) {
            Ok(res) => res,
            Err(_) => return Err("Unable to add confirmed command to queue".to_string())
        };
    }

    return  Ok("Successfully added payload to queue:".to_string());
}

fn get_scl01_contract_summary(contract: &SCL01Contract) -> Result<String, String> {
    let import = match get_contract_header(contract.contractid.clone().as_str()){
        Ok(import) => import,
        Err(_) => return Err("Unable to get contract header".to_string()),
    };

    let listings = match &contract.listings.clone() {
        Some(listings) => listings.to_owned(),
        None => HashMap::new(),
    };

    let bids = match contract.bids.clone() {
        Some(bids) => bids,
        None => HashMap::new(),
    };

    match read_contract_interactions(&contract.contractid){
        Ok(contract_interactions) => {
            let mut summary = ContractSummary::default();
            let mut total_list_val = 0;
            let mut total_bid_val: u64 = 0;
            let mut avg_fulfil_price = 0;
            let mut avg_list_price = 0;
            let mut total_fulfiled = 0;
            let mut total_listed = 0;
            let mut index = 0;
            let mut latest_fulfilled = 0;
            let mut latest_bid_value = 0;
            for fulfilment in &contract_interactions.fulfillment_summaries {
                total_list_val += fulfilment.listing_price * fulfilment.listing_amount;
                total_listed += fulfilment.listing_amount;
   
               if index == &contract_interactions.fulfillment_summaries.len() - 1 {
                    latest_fulfilled = fulfilment.bid_amount;
                    latest_bid_value = fulfilment.bid_price * fulfilment.bid_amount;
               } else {
                total_bid_val += fulfilment.bid_price * fulfilment.bid_amount;
                total_fulfiled += fulfilment.bid_amount;
               }
                index += 1;
            }

            if contract_interactions.fulfillment_summaries.len() > 1 && total_fulfiled != 0 && total_bid_val != 0{
                avg_fulfil_price = ((total_bid_val/total_fulfiled) + (latest_bid_value/latest_fulfilled))/2;
                avg_list_price = total_list_val/ total_listed; 
            } else if contract_interactions.fulfillment_summaries.len() == 1 {
                 avg_fulfil_price = latest_bid_value/latest_fulfilled;
                 avg_list_price = total_list_val/ total_listed; 
            }

            let mut available_airdrops: Option<u64> = None;
            if import.contract_type.contains("SCL02") {
                let airdrop_amount = match contract.airdrop_amount  {
                    Some(airdrop_amount) => airdrop_amount,
                    None => 0,
                };

                let total_airdrops = match contract.total_airdrops  {
                    Some(total_airdrops) => total_airdrops,
                    None => 0,
                };

                let current_airdrops = match contract.current_airdrops  {
                    Some(current_airdrops) => current_airdrops,
                    None => 0,
                };
                
		        if current_airdrops >= total_airdrops {
		            available_airdrops = Some(0);
		        }else {
		            available_airdrops = Some(airdrop_amount * (total_airdrops - current_airdrops));
		        }
            }

            summary.contract_id = contract.contractid.clone();
            summary.max_supply = contract.max_supply;
            summary.ticker = contract.ticker.clone();
            summary.rest_url = import.rest_url;
            summary.contract_type = import.contract_type;
            summary.decimals = import.decimals;
            summary.supply = contract.supply;
            summary.total_owners = contract.owners.len() as u64;
            summary.average_listing_price = avg_list_price;
            summary.average_traded_price = avg_fulfil_price;
            summary.total_listed = total_listed;
            summary.total_traded = total_fulfiled;
            summary.total_burns = contract_interactions.total_burns;
            summary.contract_interactions = contract.payloads.len() as u64;
            summary.total_transfers = contract_interactions.total_transfer_value;
            summary.current_listings = listings.len() as u64;
            summary.current_bids = bids.len() as u64;
            summary.available_airdrops = available_airdrops;
            summary.airdrop_amount = contract.airdrop_amount;
            match serde_json::to_string(&summary){
                Ok(parsed_data) => return Ok(parsed_data), 
                Err(_) => return Err("Failed to serialize contract summary".to_string())
            };            
        },
        Err(_) => return Err("Could not find contract interactions".to_string()),
    }
}

fn contruct_pagination_metadata(data: String, current_page: usize, total_pages: usize, page_entries: usize) -> String{
    let meta = PagingMetaData{ current_page, total_pages, page_entries };
    let mut result: String = "{\"data\":".to_string();
    result.push_str(&data);
    result.push_str(",");
    let meta_str = match serde_json::to_string(&meta){
        Ok(mata_str) =>  mata_str,
        Err(_) => String::new(), 
    };

    result.push_str("\"meta\":");
    result.push_str(&meta_str);
    result.push_str("}");
    return result;
}