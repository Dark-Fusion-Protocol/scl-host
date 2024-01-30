use std::collections::HashMap;
use hex::decode;
use bitcoin::{consensus::deserialize, Transaction};
use regex::Regex;
use std::fs;
use crate::{utils::{check_utxo_inputs, extract_contract_id, get_current_block_height, get_txid_from_hash, get_utxos_from_hash, handle_get_request, read_contract_interactions, read_from_file, read_server_config, replace_payload_special_characters, save_contract_interactions, write_contract_directory, write_to_file, ContractImport, FulfilledSummary, TradeTx, TxInfo}, scl01::scl01_contract::{DimAirdrop, DGE}};
use bitcoin::Address;
use super::scl01_contract::{SCL01Contract, Bid, Listing};

pub fn perform_minting_scl01(txid: &str, payload: &str) {
    match read_contract_scl01(txid, false) {
        Ok(_) => return,
        Err(_) => {},
    };
    
    if let Ok(captures) = handle_mint_payload(payload, txid) {
        let ticker = &captures.0;
        let txid_n = &captures.1;
        let max_supply = &captures.2;
        let decimals = &captures.3;

        let config = match read_server_config(){
            Ok(config) => config,
            Err(_) => return,
        };

        let bans = match config.reserved_tickers{
            Some(reserved_tickers) => reserved_tickers,
            None => Vec::new(),
        };

        if bans.contains(&ticker.to_ascii_uppercase()) {
            return;
        }

        let mut owners_map: HashMap<String, u64> = HashMap::new();
        owners_map.insert(txid_n.clone().to_string(), max_supply.clone());
        let mut payloads: HashMap<String,String> = HashMap::new();
        payloads.insert(txid.to_string(), payload.to_string());
        let new_contract = SCL01Contract {
            ticker: ticker.to_string(),
            contractid: txid.to_string(),
            supply: max_supply.clone(),
            decimals: decimals.clone() as i32,
            owners: owners_map,
            payloads: payloads,
            listings: None,
            bids: None,
            fulfillments: None,
            drips: None,
            diminishing_airdrops: None,
            dges: None,
            airdrop_amount: None,
            current_airdrops: None,
            total_airdrops: None,
            pending_claims: None,
            last_airdrop_split: None,
            right_to_mint: None,
            max_supply: Some(*max_supply),
        };
         let data = format!("{}:O-,{}", &new_contract.contractid,&max_supply);
         match fs::write(format!("./Json/UTXOS/{}.txt", &txid_n.clone()),data.clone(),){
            Ok(_) => {},
            Err(_) => return,
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                    write_contract_directory( format!("./Json/Contracts/{}/state.txt", &new_contract.contractid), s.clone(),new_contract.contractid.as_str());
                    write_contract_directory(format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),  s.clone(), new_contract.contractid.as_str());
                    let path =  "./Json/Contracts/".to_string() +"/" + &new_contract.contractid + "/header.txt";
                    let import = ContractImport{
                        contract_id: new_contract.contractid,
                        ticker: new_contract.ticker,
                        rest_url: "https://scl.darkfusion.tech/".to_string(),
                        contract_type: "SCL01".to_string(),
                        decimals: new_contract.decimals
                    };
                    let result = match serde_json::to_string(&import){
                        Ok(result) =>  result,
                        Err(_) => return, 
                    };
                    write_to_file(path, result);
            }
            Err(_) => {}
        };
    }   
}

pub fn perform_minting_scl02(txid: &str, payload: &str) {
    match read_contract_scl01(txid, false) {
        Ok(_) => return,
        Err(_) => {},
    };

    let re = match Regex::new(r"\[([^,]+),([^,]+),([^,]+),([^]]+)]") {
        Ok(re) => re,
        Err(_) => return,
    };

    if let Some(captures) = re.captures(&payload) {
        let ticker = match captures.get(1){
            Some(ticker) => ticker.as_str(),
            None => return,
        };

        let config = match read_server_config(){
            Ok(config) => config,
            Err(_) => return,
        };

        let bans = match config.reserved_tickers{
            Some(reserved_tickers) => reserved_tickers,
            None => Vec::new(),
        };

        if bans.contains(&ticker.to_ascii_uppercase()) {           
            return;
        }

        let max_supply_str = match captures.get(2){
            Some(max_supply_str) => max_supply_str.as_str(),
            None => return,
        };

        let airdrop_amount_str = match captures.get(3){
            Some(airdrop_amount_str) => airdrop_amount_str.as_str(),
            None => return,
        };

        let decimals_str = match captures.get(4){
            Some(decimals_str) => decimals_str.as_str(),
            None => return,
        };

        // Parse strings to numeric types
        let max_supply = match max_supply_str.parse(){
            Ok(max_supply) => max_supply,
            Err(_) => return,
        };

        let airdrop_amount = match airdrop_amount_str.parse(){
            Ok(airdrop_amount) => airdrop_amount,
            Err(_) => return,
        };

        let decimals = match decimals_str.parse(){
            Ok(decimals) => decimals,
            Err(_) => return,
        };

        let max_air_drops = max_supply / airdrop_amount;

        let mut payloads: HashMap<String,String> = HashMap::new();
        payloads.insert(txid.to_string(), payload.to_string());
        let new_contract = SCL01Contract {
            ticker: ticker.to_string(),
            contractid: txid.to_string(),
            supply: 0,
            airdrop_amount: Some(airdrop_amount),
            decimals: decimals,
            owners: HashMap::new(),
            pending_claims: None,
            payloads: payloads,
            listings: None,
            bids: None,
            fulfillments: None,
            total_airdrops: Some(max_air_drops),
            current_airdrops: Some(0),
            last_airdrop_split: None,
            drips: None,
            diminishing_airdrops: None,
            dges: None,
            right_to_mint: None,
            max_supply: Some(max_supply)
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                    write_contract_directory( format!("./Json/Contracts/{}/state.txt", &new_contract.contractid), s.clone(),new_contract.contractid.as_str());
                    write_contract_directory(format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),  s.clone(), new_contract.contractid.as_str());
                    let path =  "./Json/Contracts/".to_string() +"/" + &new_contract.contractid + "/header.txt";
                    let import = ContractImport{
                        contract_id: new_contract.contractid,
                        ticker: new_contract.ticker,
                        rest_url: "https://scl.darkfusion.tech/".to_string(),
                        contract_type: "SCL02".to_string(),
                        decimals: new_contract.decimals
                    };
                    let result = match serde_json::to_string(&import){
                        Ok(result) =>  result,
                        Err(_) => return, 
                    };
                    write_to_file(path, result);
            }
            Err(_) => {}
        };
    }   
}

pub fn perform_minting_scl03(txid: &str, payload: &str) {
    match read_contract_scl01(txid, false) {
        Ok(_) => return,
        Err(_) => {},
    };
    
    if let Ok(captures) = handle_mint_rtm_payload(payload, txid) {
        let config = match read_server_config(){
            Ok(config) => config,
            Err(_) => return,
        };

        let bans = match config.reserved_tickers{
            Some(reserved_tickers) => reserved_tickers,
            None => Vec::new(),
        };

        if bans.contains(&captures.0.to_ascii_uppercase()) {            
            return;
        }

        let mut max_supply = 0;
        for (_, value) in captures.2.clone() {
            max_supply += value;
        }

        let mut payloads: HashMap<String,String> = HashMap::new();
        payloads.insert(txid.to_string(), payload.to_string());
        let new_contract = SCL01Contract {
            ticker: captures.0,
            contractid: txid.to_string(),
            supply: 0,
            decimals: captures.1 as i32,
            owners: HashMap::new(),
            payloads: payloads,
            listings: None,
            bids: None,
            fulfillments: None,
            drips: None,
            diminishing_airdrops: None,
            dges: None,
            airdrop_amount: None,
            current_airdrops: None,
            total_airdrops: None,
            pending_claims: None,
            last_airdrop_split: None,
            right_to_mint: Some(captures.2),
            max_supply: Some(max_supply)
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                    write_contract_directory( format!("./Json/Contracts/{}/state.txt", &new_contract.contractid), s.clone(),new_contract.contractid.as_str());
                    write_contract_directory(format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),  s.clone(), new_contract.contractid.as_str());
                    let path =  "./Json/Contracts/".to_string() +"/" + &new_contract.contractid + "/header.txt";
                    let import = ContractImport{
                        contract_id: new_contract.contractid,
                        ticker: new_contract.ticker,
                        rest_url: "https://scl.darkfusion.tech/".to_string(),
                        contract_type: "SCL03".to_string(),
                        decimals: new_contract.decimals
                    };
                    let result = match serde_json::to_string(&import){
                        Ok(result) =>  result,
                        Err(_) => return, 
                    };
                    write_to_file(path, result);
            }
            Err(_) => {}
        };
    }   
}

pub async fn perform_rights_to_mint(txid: &str, command: &str, payload: &str, pending: bool, esplora: String){
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){       
        return;
    }
    
    let results = match handle_rtm_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    let utxos: Vec<String>  = vec![results.0.clone()];
    if !check_utxo_inputs(&utxos, &txid, esplora.clone()).await {      
        return;
    }

    let new_owners = match contract.right_to_mint(&txid.to_string(), &payload.to_string(), &results.0, &results.1, &results.2, &results.3) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);

    if !pending {
        let mut data = format!("{}:O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid,new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
        let _ = save_contract_scl01(&contract, payload, txid, false);
   }else{
        let mut data = format!("{}:P-O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DO-,{}", &contract.contractid,new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
   }
}

pub fn perform_airdrop(txid: &str,command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

     let contract_pending = match read_contract_scl01(contract_id.as_str(), true) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let p_c = match contract_pending.pending_claims.clone() {
        Some(p_c) => p_c,
        None => HashMap::new(),
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), false) {
        Ok(contract) => contract,
        Err(_) => return,
    };
    contract.pending_claims = Some(p_c);
    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }

    let words: Vec<&str> = command.split("AIRDROP").collect();
    if words.len() < 2{
        return;
    }

    let mut reciever = replace_payload_special_characters(&words[1].to_string());
    reciever = reciever.replace("TXID", txid);

    let amount = match contract.airdop(&txid.to_string(), &payload.to_string(), &reciever, pending) {
        Ok(amount) => amount,
        Err(_) => return,
    };

    if !pending {
        let data = format!("{}:O-,{}", &contract.contractid, amount);
        write_to_file(format!("./Json/UTXOS/{}.txt", &reciever),data.clone());
    } else {
        let data = format!("{}:P-C-,{}", &contract.contractid, amount);
        write_to_file(format!("./Json/UTXOS/{}.txt", &reciever),data.clone(),);
    }

    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        let _ = save_contract_scl01(&contract, payload, txid, false);
    }
}

pub fn perform_airdrop_split(mut contract: SCL01Contract){
    let new_owners = match contract.airdop_split() {
        Ok(res) => res,
        Err(_) => return,
    };

    for owner in new_owners{
        let data = format!("{}:O-,{}", &contract.contractid, owner.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &owner.0),data.clone()); 
    }

    let _ = save_contract_scl01(&contract, "Airdrop Split", "", true);
    let _ = save_contract_scl01(&contract, "Airdrop Split", "", false);
}

pub async fn perform_transfer_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }
    
    let results = match handle_transfer_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    if !check_utxo_inputs(&results.0, &txid, esplora.clone()).await {        
        return;
    }

    let block_height = match get_current_block_height(esplora).await {
        Ok(block_height) => block_height,
        Err(_) => return,
    };

    let drip = match contract.transfer(&txid.to_string(), &payload.to_string(), &results.0, &results.1, block_height as u64) {
        Ok(res) => res,
        Err(_) => return,
    };
    
    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        for s in &results.0{
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {},
                Err(_) => {},
            }
        }

        for (index, (key,value)) in results.1.iter().enumerate() {
             let mut data = format!("{}:O-,{}", &contract.contractid, value);
             if drip.0[index] && index == results.1.len() - 1 {
                data = format!("{}:DO-,{}", &contract.contractid, drip.1); 
            }else if drip.0[index]  {
                data = format!("{}:DO-,{}", &contract.contractid, value); 
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => return,
            };
        }

        let _ = save_contract_scl01(&contract, payload, txid, false);

        let mut interactions =  match read_contract_interactions(&contract_id) {
            Ok(interactions) => interactions,
            Err(_) => return,
        };

        interactions.total_transfers += 1;
        let mut total_value = 0;
        for ( _ , value) in results.1 {
            total_value += value;
        }
        interactions.total_transfer_value += total_value;
        match save_contract_interactions(&interactions, &contract_id) {
            Ok(_) => interactions,
            Err(_) => return,
        };
    } else {
           for (index, (key,value)) in results.1.iter().enumerate() {
            let mut data= format!("{}:P-O-,{}", &contract.contractid,value);
            if drip.0[index] && index == results.1.len() - 1 {
                data = format!("{}:DO-,{}", &contract.contractid, drip.1); 
            }else if drip.0[index]  {
                data = format!("{}:DO-,{}", &contract.contractid, value); 
            }

             match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => return,
            };
        }
    }
}

pub async fn perform_burn_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){      
        return;
    }

    if let Ok(result) = handle_burn_payload(txid, payload){
        if !check_utxo_inputs(&result.0, &txid, esplora).await {       
            return;
        }

        match contract.burn(&txid.to_string(), &payload.to_string(), &result.0, &result.1, &result.2) {
            Ok(_) => {},
            Err(_) => return,
        };
    
        let _ = save_contract_scl01(&contract, payload, txid, true);
        if !pending {
            let _ = save_contract_scl01(&contract, payload, txid, false);
            let mut interactions =  match read_contract_interactions(&contract_id) {
                Ok(interactions) => interactions,
                Err(_) => return,
            };
    
            interactions.total_burns += 1;
            match save_contract_interactions(&interactions, &contract_id) {
                Ok(_) => interactions,
                Err(_) => return,
            };
        }
    }
}

pub async fn perform_list_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){     
        return;
    }

    if let Ok(result) = handle_list_payload(txid, command){
        if !check_utxo_inputs(&result.0, &txid, esplora.clone()).await {    
            return;
        }

        let block_height = match get_current_block_height(esplora).await {
            Ok(block_height) => block_height,
            Err(_) => return,
        };

        let listing = Listing{
            change_utxo: result.1,
            list_utxo: result.2,
            rec_addr: result.3,
            list_amt: result.4,
            price: result.5,
            valid_bid_block: None,
        };
    
       let new_owner =  match contract.list(&txid.to_string(), &payload.to_string(), &result.0, listing.clone(), block_height as u64) {
            Ok(o) => o,
            Err(_) => return,
        };

        let _ = save_contract_scl01(&contract, payload, txid, true);
        if !pending {
            for s in &result.0{
                let file_path = format!("./Json/UTXOS/{}.txt", s);
                // Attempt to remove the file
                match fs::remove_file(file_path) {
                    Ok(_) => {},
                    Err(_) => {},
                }
            } 

            if &new_owner.1 > & 0 {
                let mut data = format!("{}:O-,{}", &contract.contractid,&new_owner.1);
                if new_owner.2 {
                    data = format!("{}:DO-,{}", &contract.contractid,&new_owner.1);
                }

                match fs::write(format!("./Json/UTXOS/{}.txt", &listing.change_utxo),data.clone(),){
                    Ok(_) => {},
                    Err(_) => return,
                };
            } 
              
            let _ = update_list_utxos(listing.clone(), contract.clone(), false,&result.0[0]);     
            let _ = save_contract_scl01(&contract, payload, txid, false);
        } else {
            if &new_owner.1 > & 0 {
                let mut data = format!("{}:P-O-,{}", &contract.contractid,&new_owner.1);
                if new_owner.2 {
                    data = format!("{}:P-DO-,{}", &contract.contractid,&new_owner.1);
                }

                write_to_file(format!("./Json/UTXOS/{}.txt", &listing.change_utxo),data.clone());
            }
            
            let _ = update_list_utxos(listing.clone(), contract.clone(), true,&result.0[0]);
        }
    }
}

fn update_list_utxos(listing: Listing, contract: SCL01Contract, pending: bool, order_id: &String) -> Result<i32, String> {
    let mut highest_bid = 0;
    let mut lowest_bid = 0;
    let mut num_bids = 0;
    let bids = match contract.bids {
        Some(b) => b,
        None => HashMap::new(),
    };

    for (_,b) in bids {
        if b.order_id == order_id.to_string() {
            num_bids += 1;
            let n = b.bid_price * b.bid_amount;
            if n > highest_bid {
                highest_bid = n;
            } else if n < lowest_bid {
                lowest_bid = n;
            } 
        }
       
    }

    let data;
    if pending{
        data = format!("{}:P-L-,{},{},{},{},{}", &contract.contractid,&listing.list_amt,&listing.price,num_bids, highest_bid, lowest_bid);
        write_to_file(format!("./Json/UTXOS/{}.txt", &listing.list_utxo),data.clone());
    } else {
        data = format!("{}:L-,{},{},{},{},{}", &contract.contractid,&listing.list_amt,&listing.price,num_bids, highest_bid, lowest_bid);
        write_to_file(format!("./Json/UTXOS/{}.txt", &listing.list_utxo),data.clone());
    }
    return Ok(0);
}

pub async fn perform_bid_scl01(txid: &str, command: &str, payload: &str, trade_txs: &Vec<TradeTx>, pending: bool, esplora: String) {    
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){       
        return;
    }
    
    let listings = match contract.listings.clone(){
        Some(listings) => listings,
        None => return,
    };

    let words: Vec<&str> = command.split("BID").collect();
    if words.len() < 2{       
        return;
    }

    let bid_split: Vec<&str> = words[1].split("],").collect(); 
    if bid_split.len() < 1 {     
        return;
    }

    let mut bids: Vec<Bid> = Vec::new();
    let mut bidding_ids: Vec<String> = Vec::new();
    let mut order_id_split = String::new();
    for split in bid_split {
        let bid_info: Vec<&str> = split.split(",").collect();
        if bid_info.len() < 4 {        
            continue;
        }
         order_id_split = replace_payload_special_characters(&bid_info[0].to_string());
        let mut accept_tx: String = "".to_string();
        let mut fulfil_tx: String = "".to_string();
        for trade_tx in trade_txs {
            if order_id_split == trade_tx.order_id {
                accept_tx = trade_tx.accept_tx.clone();
                fulfil_tx = trade_tx.fulfil_tx.clone();
                continue;
            }
        }

        let listing = match listings.get(&order_id_split){
            Some(listing) => listing,
            None =>  continue,
        };

        if accept_tx == "".to_string() || fulfil_tx == "".to_string(){        
            continue;
        }

        let amount_split = replace_payload_special_characters(&bid_info[1].to_string());
        let amt = match amount_split.parse::<u64>() {
            Ok(amt) => amt,
            Err(_) => continue
        };

        let price_split = replace_payload_special_characters(&bid_info[2].to_string());
        let price = match price_split.parse::<u64>() {
            Ok(amt) => amt,
            Err(_) => continue,
        };

        let mut res_utxo_str = replace_payload_special_characters(&bid_info[3].to_string());
        res_utxo_str = res_utxo_str.replace("TXID", txid);
        let txid = match get_txid_from_hash(&fulfil_tx){
            Ok(txid) => txid,
            Err(_) => continue,
        };

        let tx_bytes = match decode(&fulfil_tx){
            Ok(tx_bytes) => tx_bytes,
            Err(_) => continue,
        };
    
        let transaction: Transaction = match deserialize(&tx_bytes) {
            Ok(transaction) => transaction,
            Err(_) => continue,
        };

        let rec_add: Address = match listing.rec_addr.parse::<Address>(){
            Ok(a) => a,
            Err(_) => continue,
        };

        let mut total_value = 0;
        for output in  transaction.output {
            if output.script_pubkey.to_string() == rec_add.script_pubkey().to_string() {
                total_value += output.value;
            }
        }
        let payed_amt = (amt as u128 * price as u128)/10u64.pow(contract.decimals as u32) as u128;
        if total_value < payed_amt as u64 {        
            continue;
        }

        bidding_ids.push(txid);
        let fullfilment_utxos = match get_utxos_from_hash(&fulfil_tx){
            Ok(fullfilment_utxos) => fullfilment_utxos,
            Err(_) => continue,
        };

        if fullfilment_utxos.len() == 0 {      
            continue;
        }

        let bid: Bid = Bid{
            bid_amount: amt,
            bid_price: price,
            order_id: order_id_split.clone(),
            fulfill_tx: fulfil_tx.to_string(),
            accept_tx: accept_tx.to_string(),
            reseved_utxo: res_utxo_str,
            fullfilment_utxos: fullfilment_utxos,
        };
        bids.push(bid);
    }

    let block_height = match get_current_block_height(esplora).await {
        Ok(block_height) => block_height,
        Err(_) => 0
    };

    match contract.bid(&txid.to_string(), &payload.to_string(), bids.clone(), &bidding_ids, block_height) {
        Ok(_) => {},
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, &payload, &txid, true);

    let default_listings = HashMap::new();
    if !pending {
        let listings = match contract.listings{
            Some(ref p) => p,
            None => &default_listings
        };

        let l = match listings.get(&order_id_split.clone()){
            Some(listing) => listing,
            None => return,
        };

        for b in &bids{
            let data = format!("{}:B-,{},{},{},{}", &contract.contractid,b.bid_amount,b.bid_price,0.to_string(), l.list_utxo.clone());
            write_to_file(format!("./Json/UTXOS/{}.txt", b.reseved_utxo),data.clone());
        }

        let _ = save_contract_scl01(&contract, payload, txid, false);
        _ = update_list_utxos(l.clone(), contract.clone(), false,&order_id_split.clone());

    } else {
        let listings = match contract.listings{
            Some(ref p) => p,
            None => &default_listings
        };

        let l = match listings.get(&order_id_split.clone()){
            Some(listing) => listing,
            None => return,
        };

        for b in &bids{
            let data = format!("{}:P-B-,{},{},{},{}", &contract.contractid,b.bid_amount,b.bid_price,0.to_string(), l.list_utxo.clone());
            write_to_file(format!("./Json/UTXOS/{}.txt", b.reseved_utxo),data.clone());
        }
    }
}

pub async fn perform_accept_bid_scl01(txid: &str, payload: &str, pending: bool, esplora:String) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){      
        return;
    }

    let bids_available = match contract.bids.clone(){
        Some(bids_available) => bids_available,
        None => return,
    };

    let listings_available = match contract.listings.clone(){
        Some(listings_available) => listings_available,
        None => return,
    };

    let mut listing_utxos: Vec<String> = Vec::new();
    let mut bid_id: String = "".to_string();
    let mut order_id: String = "".to_string();
    for (key, value) in bids_available.clone()  {
        let accept_txid = match get_txid_from_hash(&value.accept_tx){
            Ok(accept_txid) => accept_txid,
            Err(_) => return , 
        };

        if accept_txid == txid{
            bid_id = key;
            order_id = value.order_id;
        }
    }

    if bid_id == "" || order_id == "" {    
        return;
    }

    listing_utxos.push(listings_available[&order_id].list_utxo.clone());

    if !check_utxo_inputs(&listing_utxos, &txid.to_string(), esplora).await {      
        return;
    }

    match contract.accept_bid(&txid.to_string(), &payload.to_string(), &bid_id) {
        Ok(_) => {},
        Err(_) => return,
    };

    if pending {
        let fulfill_payload = format!("{{{}:FULFIL_TRADE}}",contract_id);
        let new_owners = match contract.fulfil(&bid_id, &fulfill_payload.to_string(), &bid_id) {
            Ok(n) => n,
            Err(_) => return,
        };
         for (key,value) in &new_owners{
             let data = format!("{}:P-O-,{}", &contract.contractid,value);
             write_to_file(format!("./Json/UTXOS/{}.txt", &key),data.clone());
        }
    }

    let _ =  save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        let _ = save_contract_scl01(&contract, payload, txid, false);
    }
}

pub async fn perform_fulfil_bid_scl01(txid: &str, payload: &str, pending:bool) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };
    
    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){    
        return;
    }

    let listings = match contract.listings.clone(){
        Some(listings) => listings,
        None => return,
    };

    let bids = match contract.bids.clone(){
        Some(bids) => bids,
        None => return,
    };
    
    let fulfillments = match contract.fulfillments.clone(){
        Some(fulfillments) => fulfillments,
        None => return,
    };

    if ! fulfillments.contains_key(txid) {      
        return;
    }

    let order_id = fulfillments[txid].clone();
    let fulfillment =  FulfilledSummary{
        bid_price: bids[txid].bid_price.clone(),
        bid_amount: bids[txid].bid_amount.clone(),
        listing_amount: listings[&order_id].list_amt.clone(),
        listing_price: listings[&order_id].price.clone(),
    };

    let new_owners = match contract.fulfil(&txid.to_string(), &payload.to_string(), &txid.to_string()) {
        Ok(n) => n,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, &txid, true);

    if !pending {
        for (key,value) in &new_owners{
             let data = format!("{}:O-,{}", &contract.contractid,value);
             write_to_file(format!("./Json/UTXOS/{}.txt", &key),data.clone());
        }

        let _ = save_contract_scl01(&contract, payload, &txid, false);

        let mut interactions =  match read_contract_interactions(&contract_id) {
            Ok(interactions) => interactions,
            Err(_) => return,
        };

        interactions.fulfillment_summaries.push(fulfillment);
        match save_contract_interactions(&interactions, &contract_id) {
            Ok(_) => interactions,
            Err(_) => return,
        };
    } else { 
          for (key,value) in &new_owners{
             let data = format!("{}:P-O-,{}", &contract.contractid,value);
             write_to_file(format!("./Json/UTXOS/{}.txt", &key),data.clone());
        }
    }
}

pub async fn perform_drip_start_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){        
        return;
    }
    
    let results = match handle_drip_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };
    

     if !check_utxo_inputs(&results.0, &txid, esplora.clone()).await {    
        return;
     }

    let current_block_height = match get_current_block_height(esplora).await {
        Ok(current_block_height) => current_block_height,
        Err(_) => return,
    };

    let new_owners = match contract.start_drip(&txid.to_string(), &payload.to_string(), &results.0, &results.1, &results.2, current_block_height as u64) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        for s in &results.0{
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {},
                Err(_) => {},
            }
        } 

        for (key,value) in new_owners.0.clone(){
            let data = format!("{}:DO-,{}", &contract.contractid,value);
            match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => {} 
            };
        }
        let data = format!("{}:O-,{}", &contract.contractid,&new_owners.1.1);
        match fs::write(format!("./Json/UTXOS/{}.txt", &new_owners.1.0.clone()),data.clone(),){
           Ok(_) => {},
           Err(_) => return, 
       };

        let _ =  save_contract_scl01(&contract, payload, txid, false);
   } else {
        let data = format!("{}:P-O-,{}", &contract.contractid,&new_owners.1.1);
        match fs::write(format!("./Json/UTXOS/{}.txt", &new_owners.1.0.clone()),data.clone(),){
            Ok(_) => {},
            Err(_) => {} 
        };

         for (key,value) in new_owners.0{
            let data = format!("{}:P-DO-,{}", &contract.contractid,value);
            match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => {}
            };
       }
   }
}

pub async fn perform_create_diminishing_airdrop_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String){
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){       
        return;
    }
    
    let results = match handle_create_diminishing_airdrop_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    if !check_utxo_inputs(&results.0, &txid, esplora.clone()).await {
        return;
    }

    let current_block_height = match get_current_block_height(esplora).await {
        Ok(current_block_height) => current_block_height as u64,
        Err(_) => return,
    };

    let new_owners = match contract.create_dim_airdrop(&txid.to_string(), &payload.to_string(), &results.0, &results.1, &results.2,  &results.3, &results.4, &results.5, &results.6, &results.7, current_block_height) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        for s in &results.0{
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {},
                Err(_) => {},
            }
        } 

        let mut data = format!("{}:O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid, new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
        let _ = save_contract_scl01(&contract, payload, txid, false);
   }else{
        let mut data = format!("{}:P-O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data= format!("{}:P-DO-,{}", &contract.contractid, new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
   }
}

pub async fn perform_claim_diminishing_airdrop_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String){
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let results = match handle_claim_diminishing_airdrop_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    let contract_pending = match read_contract_scl01(contract_id.as_str(), true) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let dims = match contract_pending.diminishing_airdrops.clone(){
        Some(dims) => dims,
        None => return,
    };


    let dim:DimAirdrop = match dims.get(&results.0){
        Some(dim) => dim.clone(),
        None => return,
    };

    let mut donater_pub_address:String = String::new();

    if dim.single_drop{
        let url: String = esplora.to_string() + "tx/" + &txid;
        let response = match handle_get_request(url).await {
            Some(response) => response,
            None => return,
        };
    
        let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
            Ok(tx_info) => tx_info,
            Err(_) => return,
        };
    
        let vin = match tx_info.vin {
            Some(vin) => vin,
            None => return,
        };
        
        if vin.len() == 0 {
            return;
        }
    
        let prev_outputs = match &vin[0].prevout{
            Some(prev) => prev,
            None => return,
        };
    
        donater_pub_address = match &prev_outputs.scriptpubkey_address{
            Some(donater_pub_address) => donater_pub_address.clone(),
            None => return,
        };
    
        if dim.claimers.contains_key(&donater_pub_address){
            return;
        }
    }
    
    let p_c: HashMap<String, u64> = match contract_pending.pending_claims.clone() {
                Some(p_c) => p_c,
                None => HashMap::new(),
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    contract.pending_claims = Some(p_c);
    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }

    let new_owners = match contract.claim_dim_airdrop(&txid.to_string(), &payload.to_string(), &results.0, &results.1, pending, &donater_pub_address) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        let mut data = format!("{}:O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid,new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());

        let _ = save_contract_scl01(&contract, payload, txid, false);
   }else{
        let mut data = format!("{}:P-C-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DC-,{}", &contract.contractid,new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
   }
}

pub async fn perform_create_dge_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String){
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }
    
    let results = match handle_create_dge_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    if !check_utxo_inputs(&results.0, &txid, esplora.clone()).await {
     return;
    }

    let dge: DGE = DGE{
        pool_amount: results.1,
        sats_rate: results.2,
        max_drop: results.3,
        current_amount_dropped: 0,
        donations_address: results.5,
        drip_duration: results.4,
        donaters: HashMap::new(),
        single_drop: results.7
    };

    let current_block_height = match get_current_block_height(esplora).await {
        Ok(current_block_height) => current_block_height as u64,
        Err(_) => return,
    };

    let new_owners = match contract.create_dge(&txid.to_string(), &payload.to_string(), &results.0, dge,  &results.6, current_block_height) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);

    if !pending {
        for s in &results.0{
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {},
                Err(_) => {},
            }
        }

        let mut data = format!("{}:O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data= format!("{}:DO-,{}", &contract.contractid, new_owners.1); 
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
        let _ = save_contract_scl01(&contract, payload, txid, false);
   }else{
        let mut data = format!("{}:P-O-,{}", &contract.contractid,new_owners.1);
        if new_owners.2 {
            data= format!("{}:P-DO-,{}", &contract.contractid, new_owners.1); 
        }
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
   }
}

pub async fn perform_claim_dge_scl01(txid: &str, command: &str, payload: &str, pending: bool, esplora: String){
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };

    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }
    
    let results = match handle_claim_dge_payload(txid, command){
        Ok(results) => results,
        Err(_) => return,
    };

    let dges = match contract.dges.clone(){
        Some(dges) => dges,
        None => return,
    };

    let dge:DGE = match dges.get(&results.0){
        Some(dge) => dge.clone(),
        None => return,
    };

    let url: String = esplora.to_string() + "tx/" + &txid;
    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => return,
    };

    let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
        Ok(tx_info) => tx_info,
        Err(_) => return,
    };

    let vin = match tx_info.vin {
        Some(vin) => vin,
        None => return,
    };
    
    if vin.len() == 0 {
        return;
    }

    let prev_outputs = match &vin[0].prevout{
        Some(prev) => prev,
        None => return,
    };

    let donater_pub_address = match &prev_outputs.scriptpubkey_address{
        Some(donater_pub_address) => donater_pub_address.clone(),
        None => return,
    };
    
    let vout = match tx_info.vout {
        Some(vout) => vout,
        None => return,
    };

    let mut donation_amout = 0;
    for output in vout {
        let address = match output.scriptpubkey_address {
            Some(address) => address,
            None => continue,
        };

        let value = match output.value {
            Some(value) => value,
            None => continue,
        };

        if address == dge.donations_address {
            donation_amout += value;
        }
    }

    if donation_amout == 0 {
        return ;
    }

    if dge.single_drop && dge.donaters.contains_key(&donater_pub_address){
        return;
    }

    let current_block = match get_current_block_height(esplora.to_string()).await{
        Ok(current_block) => current_block as u64,
        Err(_) => return,
    };

    let new_owners = match contract.claim_dge(&txid.to_string(), &payload.to_string(), &results.0, &results.1, &donater_pub_address, donation_amout, current_block) {
        Ok(res) => res,
        Err(_) => return,
    };

    let _ = save_contract_scl01(&contract, payload, txid, true);
    if !pending {
        let data = format!("{}:DO-,{}", &contract.contractid,new_owners.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());

        let _ = save_contract_scl01(&contract, payload, txid, false);
   }else{
        let data = format!("{}:P-DO-,{}", &contract.contractid,new_owners.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0),data.clone());
   }
}

pub fn perform_drips_scl01(contract_id: String, block_height: u64, pending: bool){
    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return
    };

    let new_owners = match contract.drip(block_height as u64) {
        Ok(res) => res,
        Err(_) => return,
    };
    
    if new_owners.len() == 0  {
        return;
    }

    let _ = save_contract_scl01(&contract, "", "", true);

    if !pending {
        for (key,value,drip) in new_owners.clone(){
            let mut data = format!("{}:O-,{}", &contract.contractid,value);
            if drip {
                data = format!("{}:DO-,{}", &contract.contractid,value);
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => {        
                } 
            };
        }

        let _ = save_contract_scl01(&contract, "", "", false);
   } else {
        for (key,value,drip) in new_owners.clone(){
            let mut data = format!("{}:P-O-,{}", &contract.contractid,value);
            if drip {
                data = format!("{}:P-DO-,{}", &contract.contractid,value);
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key),data.clone(),){
                Ok(_) => {},
                Err(_) => {        
                } 
            };
        }
   }
}

pub async fn perform_listing_cancel_scl01(txid: &str, payload: &str, pending:bool, esplora: String) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };
    
    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){   
        return;
    }

    let words: Vec<&str> = payload.split("CANCELLISTING").collect();
    if words.len() < 2{
        return;
    }

    let listing_utxo = replace_payload_special_characters(&words[1].to_string()); 
    let utxos: Vec<String>  = vec![listing_utxo.clone()];
    if !check_utxo_inputs(&utxos, &txid, esplora.clone()).await {      
        return;
    }
       
    let file_path = format!("./Json/UTXOS/{}.txt",listing_utxo);
    // Attempt to remove the file
    match fs::remove_file(file_path) {
        Ok(_) => {},            
        Err(_) => {},
    }
     
    match contract.cancel_listing(&txid.to_string(), &listing_utxo.to_string(), payload.to_string()) {
        Ok(_) => {},
        Err(_) => return,
    }

    let _ = save_contract_scl01(&contract, payload, &txid, pending);
    if !pending {
        let _ = save_contract_scl01(&contract, payload, &txid, false);
    }
}

pub async fn perform_bid_cancel_scl01(txid: &str, payload: &str, pending:bool, esplora: String) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => return,
    };
    
    let mut contract = match read_contract_scl01(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid){
        return;
    }

    let words: Vec<&str> = payload.split("CANCELBID").collect();
    if words.len() < 2{
        return;
    }

    let bidding_utxo = replace_payload_special_characters(&words[1].to_string());
    let utxos: Vec<String>  = vec![bidding_utxo.clone()];
    if !check_utxo_inputs(&utxos, &txid, esplora.clone()).await {      
        return;
    }

    let file_path = format!("./Json/UTXOS/{}.txt", bidding_utxo);
    // Attempt to remove the file
    match fs::remove_file(file_path) {
        Ok(_) => {},            
        Err(_) => {},
    }
      
    match contract.cancel_bid(&txid.to_string(), &bidding_utxo.to_string(), payload.to_string()) {
            Ok(_) => {},
            Err(_) => return,
    }

    let _ = save_contract_scl01(&contract, payload, &txid, pending);
    if !pending {
        let _ = save_contract_scl01(&contract, payload, &txid, false);
    }
}

pub fn handle_create_diminishing_airdrop_payload(txid: &str, payload: &str)->Result<(Vec<String>, u64, u64, u64, u64, u64, String, bool), String>{
    let words: Vec<&str> = payload.split("DIMAIRDROP").collect();
    if words.len() < 2{
        return Err("Invalid dim airdrop payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 2{
        return Err("Invalid drip payload".to_string());
    }

    let sender_strs: Vec<&str> = sendsplit[0].split(",").collect();
    let mut senders: Vec<String> = Vec::new();
    for sendi in sender_strs {
        let sender = replace_payload_special_characters(&sendi.to_string());
        senders.push(sender);
    }
    let split: Vec<&str> = sendsplit[1].split(",").collect();
    if split.len() < 7{
        return Err("Invalid drip payload".to_string());
    }

    let pool_split = replace_payload_special_characters(&split[0].to_string());
    let pool = match pool_split.parse::<u64>() {
        Ok(pool) => pool,
        Err(_) => return Err("Failed to parse pool amount".to_string()),
    };

    let step_amount_split = replace_payload_special_characters(&split[1].to_string());
    let step_amount = match step_amount_split.parse::<u64>() {
        Ok(step_amount) => step_amount,
        Err(_) => return Err("Failed to parse step down amount".to_string()),
    };

    let step_period_split = replace_payload_special_characters(&split[2].to_string());
    let step_period = match step_period_split.parse::<u64>() {
        Ok(step_period) => step_period,
        Err(_) => return Err("Failed to parse min airdrop amount".to_string()),
    };

    let max_airdrop_split = replace_payload_special_characters(&split[3].to_string());
    let max_airdrop = match max_airdrop_split.parse::<u64>() {
        Ok(max_airdrop) => max_airdrop,
        Err(_) => return Err("Failed to parse max airdrop amount".to_string()),
    };

    let min_airdrop_split = replace_payload_special_characters(&split[4].to_string());
    let min_airdrop = match min_airdrop_split.parse::<u64>() {
        Ok(min_airdrop) => min_airdrop,
        Err(_) => return Err("Failed to parse min airdrop amount".to_string()),
    };

    let change_split = replace_payload_special_characters(&split[5].to_string());
    let change = change_split.replace("TXID", txid);

    let single_drop_str = replace_payload_special_characters(&split[6].to_string());
    let single_drop: bool = single_drop_str.to_ascii_lowercase().contains("true");
    return Ok((senders, pool, step_amount, step_period, max_airdrop, min_airdrop, change, single_drop));
}

pub fn handle_claim_diminishing_airdrop_payload(txid: &str, payload: &str)->Result<(String, String), String>{
    let words: Vec<&str> = payload.split("CLAIM_DIMAIRDROP").collect();
    if words.len() < 2{
        return Err("Invalid dim airdrop claim payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 2{
        return Err("Invalid dim airdrop claim payload".to_string());
    }

    let claim_id = replace_payload_special_characters(&sendsplit[0].to_string());
    let reciever_split = replace_payload_special_characters(&sendsplit[1].to_string());
    let reciever = reciever_split.replace("TXID", txid);
    return Ok((claim_id, reciever));
}

pub fn handle_claim_dge_payload(txid: &str, payload: &str)->Result<(String, String), String>{
    let words: Vec<&str> = payload.split("CLAIM_DGE").collect();
    if words.len() < 2{
        return Err("Invalid dge claim payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 2{
        return Err("Invalid dge claim payload".to_string());
    }

    let claim_id = replace_payload_special_characters(&sendsplit[0].to_string());
    let reciever_split = replace_payload_special_characters(&sendsplit[1].to_string());
    let reciever = reciever_split.replace("TXID", txid);
    return Ok((claim_id, reciever));
}

pub fn handle_create_dge_payload(txid: &str, payload: &str)->Result<(Vec<String>, u64, u64, u64, u64, String, String, bool), String>{
    let words: Vec<&str> = payload.split("DGE").collect();
    if words.len() < 2{
        return Err("Invalid dge creation payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 2{
        return Err("Invalid dge creation payload".to_string());
    }

    let sender_strs: Vec<&str> = sendsplit[0].split(",").collect();
    let mut senders: Vec<String> = Vec::new();
    for sendi in sender_strs {
        let sender = replace_payload_special_characters(&sendi.to_string());
        senders.push(sender);
    }
    let split: Vec<&str> = sendsplit[1].split(",").collect();
    if split.len() < 7 {
        return Err("Invalid dge creation payload".to_string());
    }

    let pool_split = replace_payload_special_characters(&split[0].to_string());
    let pool = match pool_split.parse::<u64>() {
        Ok(pool) => pool,
        Err(_) => return Err("Failed to parse pool amount".to_string()),
    };

    let sats_rate_split = replace_payload_special_characters(&split[1].to_string());
    let sats_rate = match sats_rate_split.parse::<u64>() {
        Ok(sats_rate) => sats_rate,
        Err(_) => return Err("Failed to parse sats rate".to_string()),
    };

    let max_drop_split = replace_payload_special_characters(&split[2].to_string());
    let max_drop = match max_drop_split.parse::<u64>() {
        Ok(max_drop) => max_drop,
        Err(_) => return Err("Failed to parse max drop".to_string()),
    };

    let drip_duration_split = replace_payload_special_characters(&split[3].to_string());
    let drip_duration = match drip_duration_split.parse::<u64>() {
        Ok(drip_duration) => drip_duration,
        Err(_) => return Err("Failed to parse drip duration".to_string()),
    };

    let address_split = replace_payload_special_characters(&split[4].to_string());

    let change_split = replace_payload_special_characters(&split[5].to_string());
    let change = change_split.replace("TXID", txid);

    let single_drop_str = replace_payload_special_characters(&split[6].to_string());
    let single_drop: bool = single_drop_str.to_ascii_lowercase().contains("true");
    return Ok((senders, pool, sats_rate, max_drop, drip_duration, address_split, change, single_drop));
}

pub fn read_contract_scl01(contract_id: &str, pending: bool) -> Result<SCL01Contract, String> {
    let mut path = "./Json/Contracts/".to_string() +"/" + contract_id + "/state.txt";
    if pending  {
        path =  "./Json/Contracts/".to_string() +"/" + contract_id + "/pending.txt";
    }
    match read_from_file(path) {
        Some(contract_obj) => {
            let parsed_data: Result<SCL01Contract, serde_json::Error> = serde_json::from_str(&contract_obj);
            match parsed_data {
                Ok(data) => return Ok(data),
                Err(_) => return Err("Failed to deserialize contract".to_string())
            }
        }
        None => return Err("Could not find contract".to_string()),
    }
}

pub fn save_contract_scl01(contract: &SCL01Contract, _payload: &str, _txid: &str, pending: bool) -> Result<String, String> {
    let path: String;
    if !pending{
        path = format!("{}/{}/state.txt", "./Json/Contracts/", contract.contractid.to_string());
    }else{
        path = format!("{}/{}/pending.txt", "./Json/Contracts/", contract.contractid);
    }
    
    match serde_json::to_string(&contract) {
        Ok(state_string) => write_to_file(path, state_string),
        Err(_) => return Err("Failed to save updated contract".to_string()),
    };

    return Ok("Success".to_string());
}

pub fn handle_mint_payload(payload: &str, txid: &str) -> Result<(String,String, u64, u64), String>{
    let mut mint_strings: Vec<String> = Vec::new();
    let mut mint_values: Vec<u64> = Vec::new();
    let re = match Regex::new(r"\[([^,]+),([^,]+),([^,]+),([^]]+)]") {
        Ok(re) => re,
        Err(_) => return Err("Not mint valid payload".to_string()),
    };

    if let Some(captures) = re.captures(payload) {
        let ticker = match captures.get(1){
            Some(ticker) => ticker.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let max_supply_str = match captures.get(2){
            Some(max_supply_str) => max_supply_str.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let decimals_str = match captures.get(3){
            Some(decimals_str) => decimals_str.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let txid_n = match captures.get(4){
            Some(txid_n) => txid_n.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let temp: Vec<_> = txid_n.split(":").collect();
        let mut t_n = txid_n.to_string();
        if temp.len() == 2 {
            let index = temp[1];
            t_n = format!("{}:{}", &txid, index);
        }


        // Parse strings to numeric types
        let max_supply = match max_supply_str.parse(){
            Ok(max_supply) => max_supply,
            Err(_) => return Err("Not mint valid payload".to_string()),
        };

        let decimals = match decimals_str.parse(){
            Ok(decimals) => decimals,
            Err(_) => return Err("Not mint valid payload".to_string()),
        };

        mint_strings.push(ticker.to_string());
        mint_strings.push(t_n.to_string());
        mint_values.push(max_supply);
        mint_values.push(decimals);
        return Ok((ticker.to_string(),t_n, max_supply, decimals));
    }
    return Err("Not mint valid payload".to_string())
}

pub fn handle_mint_rtm_payload(payload: &str, txid: &str) -> Result<(String, u64, HashMap<String, u64>), String>{
    let words: Vec<&str> = payload.split("SCL03:").collect();
    if words.len() < 2{
        return Err("Invalid transfer payload".to_string());
    }

    let mint_split: Vec<&str> = words[1].split(",").collect();
    if mint_split.len() < 2{
        return Err("Invalid mint rtm payload".to_string());
    }

    let ticker = replace_payload_special_characters(&mint_split[0].to_string());
    let decimal_split = replace_payload_special_characters(&mint_split[1].to_string());

    // Parse strings to numeric types
    let decimals = match decimal_split.parse(){
        Ok(decimals) => decimals,
        Err(_) => return Err("Not mint valid payload".to_string()),
    };

    let rights_split: Vec<&str> = words[1].split(",[").collect();
    if rights_split.len() < 2 {
        return Err("Invalid mint rtm payload".to_string());
    }

    let rights: Vec<&str> = rights_split[1].split(",").collect();
    let mut rights_recievers: HashMap<String, u64> = HashMap::new();
    if rights.len() < 1 {
        return Err("Invalid mint rtm payload".to_string());
    }
    for reci in rights {
        let rec_str = reci.replace("TXID", txid);
        let recievers = replace_payload_special_characters(&rec_str.to_string());       
        let data: Vec<&str> = recievers.split("(").collect();
        if data.len() < 2{
            return Err("Invalid Mint payload".to_string());
        }

        let rec_utxo = data[0];
        let rec_amt_str = data[1].replace(")", "");
        let rec_amt = match rec_amt_str.parse::<u64>() {
            Ok(rec_amt) => rec_amt,
            Err(_) => return Err("Failed to parse recieved amount".to_string()),
        };

        rights_recievers.insert(rec_utxo.to_string(), rec_amt);
    }

    return Ok((ticker.to_string(), decimals, rights_recievers));
}

pub fn handle_rtm_payload(txid: &str, payload: &str)->Result<(String, String, String, u64), String>{
    let words: Vec<&str> = payload.split("RIGHTTOMINT").collect();
    if words.len() < 2{
        return Err("Invalid rtm payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 4{
        return Err("Invalid rtm payload".to_string());
    }

    let rights_utxo = replace_payload_special_characters(&sendsplit[0].to_string());
    let reciever_split = replace_payload_special_characters(&sendsplit[1].to_string());
    let reciever = reciever_split.replace("TXID", txid);
    let change_split = replace_payload_special_characters(&sendsplit[2].to_string());
    let change = change_split.replace("TXID", txid);
    let mint_amt_split = replace_payload_special_characters(&sendsplit[3].to_string());
    let mint_amt = match mint_amt_split.parse::<u64>() {
        Ok(mint_amt_split) => mint_amt_split,
        Err(_) => return Err("Failed to parse mint amount".to_string()),
    };

    return Ok((rights_utxo, reciever, change, mint_amt));
}

pub fn handle_transfer_payload(txid: &str, payload: &str) -> Result<(Vec<String>, Vec<(String, u64)>, String), String> {
    let words: Vec<&str> = payload.split("TRANSFER").collect();
    if words.len() < 2{
        return Err("Invalid transfer payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],[").collect();
    if sendsplit.len() < 2{
        return Err("Invalid transfer payload".to_string());
    }

    let sender_strs: Vec<&str> = sendsplit[0].split(",").collect();
    let rec_split: Vec<&str> = sendsplit[1].split(",").collect();
    let mut senders: Vec<String> = Vec::new();
    for sendi in sender_strs {
        let sender = replace_payload_special_characters(&sendi.to_string());
        senders.push(sender);
    }   
    let mut last_output = String::new();
    let mut rec_dict: Vec<(String, u64)> = Vec::new();
    for reci in &rec_split {
        let rec_str = reci.replace("TXID", txid);
        let recievers = replace_payload_special_characters(&rec_str.to_string());       
        let data: Vec<&str> = recievers.split("(").collect();
        if data.len() < 2{
            return Err("Invalid transfer payload".to_string());
        }
        let rec_utxo = data[0];
        let rec_amt_str = data[1].replace(")", "");
        let rec_amt = match rec_amt_str.parse::<u64>() {
            Ok(rec_amt) => rec_amt,
            Err(_) => {
                return Err("Failed to parse recieved amount".to_string());
            }
        };
        if &rec_split.last().unwrap() == &reci {
            last_output = rec_utxo.to_string();
        } 
        rec_dict.push((rec_utxo.to_string(), rec_amt));
    }

    return Ok((senders, rec_dict, last_output));
}

pub fn handle_drip_payload(txid: &str, payload: &str) -> Result<(Vec<String>, HashMap<String, (u64,u64)>, String), String> {
    let words: Vec<&str> = payload.split("DRIP").collect();
    if words.len() < 2{
        return Err("Invalid drip payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 3{
        return Err("Invalid drip payload".to_string());
    }

    let sender_strs: Vec<&str> = sendsplit[0].split(",").collect();
    let rec_split: Vec<&str> = sendsplit[1].split("),").collect();
    let change_split = replace_payload_special_characters(&sendsplit[2].to_string());
    let change = change_split.replace("TXID", txid);
    let mut senders: Vec<String> = Vec::new();
    for sendi in sender_strs {
        let sender = replace_payload_special_characters(&sendi.to_string());
        senders.push(sender);
    }

    let mut rec_dict: HashMap<String, (u64,u64)> = HashMap::new();
    for reci in rec_split {
        let rec_str = reci.replace("TXID", txid);
        let recievers = replace_payload_special_characters(&rec_str.to_string());       
        let data: Vec<&str> = recievers.split("(").collect();
        if data.len() < 2{
            return Err("Invalid Drip payload".to_string());
        }

        let rec_utxo = data[0];
        let rec_amt_str = data[1].replace(")", "");
        let tuple_str: Vec<_> = rec_amt_str.split(",").collect();
        if tuple_str.len() < 2 {
            continue;
        }
        let rec_amt = match tuple_str[0].parse::<u64>() {
            Ok(rec_amt) => rec_amt,
            Err(_) => return Err("Failed to parse recieved amount".to_string()),
        };
        let dur_amt = match tuple_str[1].parse::<u64>() {
            Ok(dur_amt) => dur_amt,
            Err(_) => return Err("Failed to parse duration amount".to_string()),
        };
        
        rec_dict.insert(rec_utxo.to_string(), (rec_amt, dur_amt));
    }
    return Ok((senders, rec_dict, change));
}

pub fn handle_burn_payload(txid: &str, payload: &str) -> Result<(Vec<String>, u64, String), String> {
    let words: Vec<&str> = payload.split("BURN").collect();
    if words.len() < 2 {
        return Err("Invalid burn payload".to_string());
    }

    let burn_split: Vec<&str> = words[1].split("],").collect(); 
    if burn_split.len() < 2 {
        return Err("Invalid burn payload".to_string());
    }
    let burners_split: Vec<&str> = burn_split[0].split(",").collect();
    let mut burners: Vec<String> = Vec::new();
    for burner in burners_split.clone() {
        let clean_burner = replace_payload_special_characters(&burner.to_string());
        burners.push(clean_burner);
    }

    let burn_info: Vec<&str> = burn_split[1].split(",").collect();
    if burn_info.len() < 2 {
        return Err("Invalid burn payload".to_string());
    } 

    let burn_amount_str = replace_payload_special_characters(&burn_info[0].to_string());

    let amt = match burn_amount_str.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => return Err("Invalid burn payload".to_string()),
    };

    let mut change_str = replace_payload_special_characters(&burn_info[1].to_string());
    change_str = change_str.replace("TXID", txid);
     for s in &burners{
        let file_path = format!("./Json/UTXOS/{}.txt", s);
        // Attempt to remove the file
        match fs::remove_file(file_path) {
            Ok(_) => {},            
            Err(_) => {}
        }
    }
    return Ok((burners, amt, change_str));
}

pub fn handle_list_payload(txid: &str, payload: &str) -> Result<(Vec<String>, String, String, String, u64, u64), String>{
    let words: Vec<&str> = payload.split("LIST").collect();
    if words.len() < 2 {
        return Err("Invalid List payload".to_string());
    }

    let list_split: Vec<&str> = words[1].split("],").collect(); 
    if list_split.len() < 2 {
        return Err("Invalid List payload".to_string());
    }

    let lists_split: Vec<&str> = list_split[0].split(",").collect();
    let listing_info: Vec<&str> = list_split[1].split(",").collect();
    let mut listings_senders: Vec<String> = Vec::new();
    for lising in lists_split.clone() {
        let temp1 = lising.to_string().replace("[", "");
        let temp2 = temp1.replace("]", "");
        listings_senders.push(temp2);
    }

    if listing_info.len() < 5 {
        return Err("Invalid List payload".to_string());
    }

    let mut change_str = replace_payload_special_characters(&listing_info[0].to_string());
    change_str = change_str.replace("TXID", txid);
    let mut listing_utxo_str = replace_payload_special_characters(&listing_info[1].to_string());
    listing_utxo_str = listing_utxo_str.replace("TXID", txid);
    let list_amount_str = replace_payload_special_characters(&listing_info[2].to_string());
    let sell_price_str = replace_payload_special_characters(&listing_info[3].to_string());
    let pay_address_str = replace_payload_special_characters(&listing_info[4].to_string());
    let listing_amt = match list_amount_str.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => return Err("Invalid List payload. Listing amount invalid".to_string()),
    };

    let sell_price = match sell_price_str.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => return Err("Invalid List payload. Sell price invalid".to_string()),
    };
    
    return Ok((listings_senders, change_str, listing_utxo_str, pay_address_str, listing_amt, sell_price));
}

 pub fn handle_bid_payload(txid: &str, payload: &str) -> Result<Vec<(String, u64, u64, String)>, String>{
     let words: Vec<&str> = payload.split("BID").collect();
     let bid_split: Vec<&str> = words[1].split("],").collect(); 
     if bid_split.len() < 1 {
         return Err("Invalid Bid payload. Sell price invalid".to_string());
     }
 
     let mut bid_results: Vec<(String, u64, u64, String)> =  Vec::new();
     for split in bid_split {
         let bid_info: Vec<&str> = split.split(",").collect();
         if bid_info.len() < 4 {
             continue;
         }
 
         let order_id_split = replace_payload_special_characters(&bid_info[0].to_string());
         let amount_split = replace_payload_special_characters(&bid_info[1].to_string());
         let amt = match amount_split.parse::<u64>() {
             Ok(amt) => amt,
             Err(_) => continue,
         };
 
         let price_split = replace_payload_special_characters(&bid_info[2].to_string());
         let price = match price_split.parse::<u64>() {
             Ok(amt) => amt,
             Err(_) => continue,
         };
 
         let mut res_utxo_str = replace_payload_special_characters(&bid_info[3].to_string());
         res_utxo_str = res_utxo_str.replace("TXID", txid);
         bid_results.push((order_id_split, amt, price, res_utxo_str));
     }
     
     return Ok(bid_results);
 }

 pub fn handle_payload_extra_trade_info(payload: &str) -> Result<(String, u64, u64), String>{
    let words: Vec<&str> = payload.split("-ExtraInfo-").collect();
    if words.len() < 2 {
        return Err("Invalid payload".to_string());
    }
    let bid_split: Vec<&str> = words[1].split(",").collect(); 
    if bid_split.len() < 3 {
        return Err("Invalid payload".to_string());
    }

    let bid_id_split = replace_payload_special_characters(&bid_split[0].to_string());
    let amount_split = replace_payload_special_characters(&bid_split[1].to_string());
    let amount = match amount_split.parse::<u64>() {
        Ok(amount) => amount,
        Err(_) => return Err("Invalid payload".to_string()),
    };

    let price_split = replace_payload_special_characters(&bid_split[2].to_string());
    let price = match price_split.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => return Err("Invalid payload".to_string()),
    };
    
    return Ok((bid_id_split, amount, price));
}

pub fn convert_old_contracts() {
    let directory_path = "./Json/Contracts/"; // Your directory path here
    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Some(folder_name) = entry.file_name().into_string().ok() {
                        let file_path = format!("./Json/Contracts/{}/state.txt",&folder_name);
                        let user_data_str = match fs::read_to_string(&file_path){
                        Ok(user_data_str) => user_data_str,
                        Err(_) => continue,
                        };
                      
                        // Deserialize user data from JSON
                        let mut user_data: SCL01Contract = match serde_json::from_str(&user_data_str){
                            Ok(user_data) => user_data,
                            Err(_) => continue,
                        };

                        let airdrop_amount = match user_data.airdrop_amount  {
                            Some(airdrop_amount) => airdrop_amount,
                            None => continue,
                        };
      
                        let current_airdrops = match user_data.current_airdrops  {
                            Some(current_airdrops) => current_airdrops,
                            None => continue,
                        };

                        let total_airdrops = match user_data.total_airdrops  {
                            Some(total_airdrops) => total_airdrops,
                            None => continue,
                        };

                        user_data.max_supply = Some(total_airdrops * airdrop_amount);
                        user_data.supply = current_airdrops * airdrop_amount;

                        let serialised_user_data = match serde_json::to_string(&user_data){
                            Ok(serialised_user_data) => serialised_user_data,
                            Err(_) => continue,
                        };

                        write_to_file(format!("./Json/Contracts/{}/state.txt",&folder_name), serialised_user_data.clone());
                        write_to_file(format!("./Json/Contracts/{}/pending.txt",&folder_name),serialised_user_data,);
                 
                    }
                  }
              }
          }
      }
}