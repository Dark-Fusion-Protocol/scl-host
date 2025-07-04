use crate::utils::record_failed_transaction;
use super::scl01_contract::{Bid, LiquidityPool, Listing, SCL01Contract};
use crate::{
    scl01::scl01_contract::{DimAirdrop, DGE},
    utils::{
        check_utxo_inputs, extract_contract_id, get_current_block_height, get_tx_inputs,
        get_txid_from_hash, get_utxos_from_hash, handle_get_request, read_contract_interactions,
        read_from_file, read_server_config, read_server_lookup, replace_payload_special_characters,
        save_contract_interactions, save_server_lookup, write_contract_directory, write_to_file,
        Config, ContractImport, FulfilledSummary, Lookups, TradeTx, TxInfo,
    },
};
use bitcoin::{consensus::deserialize, Address, Transaction};
use hex::decode;
use regex::Regex;
use std::collections::HashMap;
use std::fs;

pub fn perform_minting_scl01(txid: &str, payload: &str) {

    match read_contract(txid, false) {
        Ok(_) => return,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
        }
    };

    if let Ok(captures) = handle_mint_payload(payload, txid) {
        let ticker = &captures.0;
        let txid_n = &captures.1;
        let max_supply = &captures.2;
        let decimals = &captures.3;

        let mut owners_map: HashMap<String, u64> = HashMap::new();
        owners_map.insert(txid_n.clone().to_string(), max_supply.clone());
        let mut payloads: HashMap<String, String> = HashMap::new();
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
            liquidated_tokens: None,
            liquidity_pool: None,
            token_data: None,
        };
        let data = format!("{}:O-,{}", &new_contract.contractid, &max_supply);
        match fs::write(
            format!("./Json/UTXOS/{}.txt", &txid_n.clone()),
            data.clone(),
        ) {
            Ok(_) => {}
            Err(_) => {
                record_failed_transaction(txid, "write_utxo_failed");
                return;
            }
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                write_contract_directory(
                    format!("./Json/Contracts/{}/state.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                write_contract_directory(
                    format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                let path = "./Json/Contracts/".to_string()
                    + "/"
                    + &new_contract.contractid
                    + "/header.txt";
                let config = match read_server_config() {
                    Ok(config) => config,
                    Err(_) => Config::default(),
                };

                let url = match config.url {
                    Some(url) => url,
                    None => "https://scl.darkfusion.tech/".to_owned(),
                };

                let import = ContractImport {
                    contract_id: new_contract.contractid,
                    ticker: new_contract.ticker,
                    rest_url: url.to_string(),
                    contract_type: "SCL01".to_string(),
                    decimals: new_contract.decimals,
                };
                let result = match serde_json::to_string(&import) {
                    Ok(result) => result,
                    Err(_) => {
                        record_failed_transaction(txid, "import_to_string_failed");
                        return;
                    }
                };
                write_to_file(path, result);
            }
            Err(_) => {
                record_failed_transaction(txid, "contract_to_string_failed");
            }
        };
    }
}

pub fn perform_minting_scl02(txid: &str, payload: &str) {

    match read_contract(txid, false) {
        Ok(_) => return,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
        }
    };


    let re = match Regex::new(r"\[([^,]+),([^,]+),([^,]+),([^]]+)]") {
        Ok(re) => re,
        Err(_) => {
            record_failed_transaction(txid, "regex_failed");
            return;
        }
    };

    if let Some(captures) = re.captures(&payload) {

        let ticker = match captures.get(1) {
            Some(ticker) => ticker.as_str(),
            None => {
                record_failed_transaction(txid, "ticker_parse_failed");
                return;
            }
        };

        let max_supply_str = match captures.get(2) {
            Some(max_supply_str) => max_supply_str.as_str(),
            None => {
                record_failed_transaction(txid, "max_supply_parse_failed");
                return;
            }
        };

        let airdrop_amount_str = match captures.get(3) {
            Some(airdrop_amount_str) => airdrop_amount_str.as_str(),
            None => {
                record_failed_transaction(txid, "airdrop_amount_parse_failed");
                return;
            }
        };

        let decimals_str = match captures.get(4) {
            Some(decimals_str) => decimals_str.as_str(),
            None => {
                record_failed_transaction(txid, "decimals_parse_failed");
                return;
            }
        };

        // Parse strings to numeric types

        let max_supply = match max_supply_str.parse() {
            Ok(max_supply) => max_supply,
            Err(_) => {
                record_failed_transaction(txid, "max_supply_parse_failed");
                return;
            }
        };

        let airdrop_amount = match airdrop_amount_str.parse() {
            Ok(airdrop_amount) => airdrop_amount,
            Err(_) => {
                record_failed_transaction(txid, "airdrop_amount_parse_failed");
                return;
            }
        };

        let decimals = match decimals_str.parse() {
            Ok(decimals) => decimals,
            Err(_) => {
                record_failed_transaction(txid, "decimals_parse_failed");
                return;
            }
        };

        let max_air_drops = max_supply / airdrop_amount;

        let mut payloads: HashMap<String, String> = HashMap::new();
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
            max_supply: Some(max_supply),
            liquidated_tokens: None,
            liquidity_pool: None,
            token_data: None,
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                write_contract_directory(
                    format!("./Json/Contracts/{}/state.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                write_contract_directory(
                    format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                let config = match read_server_config() {
                    Ok(config) => config,
                    Err(_) => Config::default(),
                };

                let url = match config.url {
                    Some(url) => url,
                    None => "https://scl.darkfusion.tech/".to_owned(),
                };

                let path = "./Json/Contracts/".to_string()
                    + "/"
                    + &new_contract.contractid
                    + "/header.txt";
                let import = ContractImport {
                    contract_id: new_contract.contractid,
                    ticker: new_contract.ticker,
                    rest_url: url,
                    contract_type: "SCL02".to_string(),
                    decimals: new_contract.decimals,
                };
                let result = match serde_json::to_string(&import) {
                    Ok(result) => result,
                    Err(_) => {
                        record_failed_transaction(txid, "import_to_string_failed");
                        return;
                    }
                };
                write_to_file(path, result);
            }
            Err(_) => {
                record_failed_transaction(txid, "contract_to_string_failed");
            }
        };
    }
}

pub fn perform_minting_scl03(txid: &str, payload: &str) {

    match read_contract(txid, false) {
        Ok(_) => return,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
        }
    };

    if let Ok(captures) = handle_mint_rtm_payload(payload, txid) {
        let mut max_supply = 0;
        for (_, value) in captures.2.clone() {
            max_supply += value;
        }

        let mut payloads: HashMap<String, String> = HashMap::new();
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
            max_supply: Some(max_supply),
            liquidated_tokens: None,
            liquidity_pool: None,
            token_data: None,
        };

        match serde_json::to_string(&new_contract) {
            Ok(s) => {
                write_contract_directory(
                    format!("./Json/Contracts/{}/state.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                write_contract_directory(
                    format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),
                    s.clone(),
                    new_contract.contractid.as_str(),
                );
                let config = match read_server_config() {
                    Ok(config) => config,
                    Err(_) => Config::default(),
                };

                let url = match config.url {
                    Some(url) => url,
                    None => "https://scl.darkfusion.tech/".to_owned(),
                };

                let path = "./Json/Contracts/".to_string()
                    + "/"
                    + &new_contract.contractid
                    + "/header.txt";
                let import = ContractImport {
                    contract_id: new_contract.contractid,
                    ticker: new_contract.ticker,
                    rest_url: url,
                    contract_type: "SCL03".to_string(),
                    decimals: new_contract.decimals,
                };
                let result = match serde_json::to_string(&import) {
                    Ok(result) => result,
                    Err(_) => {
                        record_failed_transaction(txid, "import_to_string_failed");
                        return;
                    }
                };
                write_to_file(path, result);
            }
            Err(_) => {
                record_failed_transaction(txid, "contract_to_string_failed");
            }
        };
    }
}

pub async fn perform_rights_to_mint(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_rtm_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_rtm_payload_failed");
            return;
        }
    };

    let utxos: Vec<String> = vec![results.0.clone()];
    if !check_utxo_inputs(&utxos, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let new_owners = match contract.right_to_mint(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        &results.2,
        &results.3,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "contract_right_to_mint_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);

    if !pending {
        let mut data = format!("{}:O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let mut data = format!("{}:P-O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
    }
}

pub fn perform_airdrop(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let contract_pending = match read_contract(contract_id.as_str(), true) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_pending_failed");
            return;
        }
    };

    let p_c = match contract_pending.pending_claims.clone() {
        Some(p_c) => p_c,
        None => HashMap::new(),
    };

    let mut contract = match read_contract(contract_id.as_str(), false) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };
    contract.pending_claims = Some(p_c);
    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let words: Vec<&str> = command.split("AIRDROP").collect();
    if words.len() < 2 {
        record_failed_transaction(txid, "malformed_airdrop_command");
        return;
    }

    let mut reciever = replace_payload_special_characters(&words[1].to_string());
    reciever = reciever.replace("TXID", txid);

    let amount = match contract.airdop(&txid.to_string(), &payload.to_string(), &reciever, pending)
    {
        Ok(amount) => amount,
        Err(_) => {
            record_failed_transaction(txid, "airdrop_failed");
            return;
        }
    };

    if !pending {
        let data = format!("{}:O-,{}", &contract.contractid, amount);
        write_to_file(format!("./Json/UTXOS/{}.txt", &reciever), data.clone());
    } else {
        let data = format!("{}:P-C-,{}", &contract.contractid, amount);
        write_to_file(format!("./Json/UTXOS/{}.txt", &reciever), data.clone());
    }

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        let _ = save_contract(&contract, payload, txid, false);
    }
}

pub fn perform_airdrop_split(mut contract: SCL01Contract) {
    let new_owners = match contract.airdop_split() {
        Ok(res) => res,
        Err(_) => {
            // No txid available, so we can't record_failed_transaction here
            return;
        }
    };

    for owner in new_owners {
        let data = format!("{}:O-,{}", &contract.contractid, owner.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &owner.0), data.clone());
    }

    let _ = save_contract(&contract, "", "", true);
    let _ = save_contract(&contract, "", "", false);
}

pub async fn perform_transfer(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_transfer_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_transfer_payload_failed");
            return;
        }
    };

    if !check_utxo_inputs(&results.0, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let block_height = match get_current_block_height().await {
        Ok(block_height) => block_height,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            return;
        }
    };

    let drip = match contract.transfer(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        block_height as u64,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "contract_transfer_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        for s in &results.0 {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        for (index, (key, value)) in results.1.iter().enumerate() {
            let mut data = format!("{}:O-,{}", &contract.contractid, value);
            if drip.0[index] && index == results.1.len() - 1 {
                data = format!("{}:DO-,{}", &contract.contractid, drip.1);
            } else if drip.0[index] {
                data = format!("{}:DO-,{}", &contract.contractid, value);
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {
                    record_failed_transaction(txid, "write_utxo_failed");
                    return;
                }
            };
        }

        let _ = save_contract(&contract, payload, txid, false);

        let mut interactions = match read_contract_interactions(&contract_id) {
            Ok(interactions) => interactions,
            Err(_) => {
                record_failed_transaction(txid, "read_contract_interactions_failed");
                return;
            }
        };

        interactions.total_transfers += 1;
        let mut total_value = 0;
        for (_, value) in results.1 {
            total_value += value;
        }
        interactions.total_transfer_value += total_value;
        match save_contract_interactions(&interactions, &contract_id) {
            Ok(_) => interactions,
            Err(_) => {
                record_failed_transaction(txid, "save_contract_interactions_failed");
                return;
            }
        };
    } else {
        for (index, (key, value)) in results.1.iter().enumerate() {
            let mut data = format!("{}:P-O-,{}", &contract.contractid, value);
            if drip.0[index] && index == results.1.len() - 1 {
                data = format!("{}:DO-,{}", &contract.contractid, drip.1);
            } else if drip.0[index] {
                data = format!("{}:DO-,{}", &contract.contractid, value);
            }

            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {
                    record_failed_transaction(txid, "write_utxo_failed_pending");
                    return;
                }
            };
        }
    }
}

pub async fn perform_burn(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    if let Ok(result) = handle_burn_payload(txid, payload) {
        if !check_utxo_inputs(&result.0, &txid).await {
            record_failed_transaction(txid, "check_utxo_inputs_failed");
            return;
        }

        match contract.burn(
            &txid.to_string(),
            &payload.to_string(),
            &result.0,
            &result.1,
            &result.2,
        ) {
            Ok(_) => {}
            Err(_) => {
                record_failed_transaction(txid, "contract_burn_failed");
                return;
            }
        };

        let _ = save_contract(&contract, payload, txid, true);
        if !pending {
            let _ = save_contract(&contract, payload, txid, false);
            let mut interactions = match read_contract_interactions(&contract_id) {
                Ok(interactions) => interactions,
                Err(_) => {
                    record_failed_transaction(txid, "read_contract_interactions_failed");
                    return;
                }
            };

            interactions.total_burns += 1;
            match save_contract_interactions(&interactions, &contract_id) {
                Ok(_) => interactions,
                Err(_) => {
                    record_failed_transaction(txid, "save_contract_interactions_failed");
                    return;
                }
            };
        }
    } else {
        record_failed_transaction(txid, "handle_burn_payload_failed");
    }
}

pub async fn perform_list(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    if let Ok(result) = handle_list_payload(txid, command) {
        if !check_utxo_inputs(&result.0, &txid).await {
            record_failed_transaction(txid, "check_utxo_inputs_failed");
            return;
        }

        let block_height = match get_current_block_height().await {
            Ok(block_height) => block_height,
            Err(_) => {
                record_failed_transaction(txid, "get_current_block_height_failed");
                return;
            }
        };

        let listing = Listing {
            change_utxo: result.1,
            list_utxo: result.2,
            rec_addr: result.3,
            list_amt: result.4,
            price: result.5,
            valid_bid_block: None,
        };

        let new_owner = match contract.list(
            &txid.to_string(),
            &payload.to_string(),
            &result.0,
            listing.clone(),
            block_height as u64,
        ) {
            Ok(o) => o,
            Err(_) => {
                record_failed_transaction(txid, "contract_list_failed");
                return;
            }
        };

        let _ = save_contract(&contract, payload, txid, true);
        if !pending {
            for s in &result.0 {
                let file_path = format!("./Json/UTXOS/{}.txt", s);
                // Attempt to remove the file
                match fs::remove_file(file_path) {
                    Ok(_) => {}
                    Err(_) => {
                        record_failed_transaction(txid, "remove_utxo_file_failed");
                    }
                }
            }

            if &new_owner.1 > &0 {
                let mut data = format!("{}:O-,{}", &contract.contractid, &new_owner.1);
                if new_owner.2 {
                    data = format!("{}:DO-,{}", &contract.contractid, &new_owner.1);
                }

                match fs::write(
                    format!("./Json/UTXOS/{}.txt", &listing.change_utxo),
                    data.clone(),
                ) {
                    Ok(_) => {}
                    Err(_) => {
                        record_failed_transaction(txid, "write_utxo_file_failed");
                        return;
                    }
                };
            }

            let _ = update_list_utxos(listing.clone(), contract.clone(), false, &result.0[0]);
            let _ = save_contract(&contract, payload, txid, false);
        } else {
            if &new_owner.1 > &0 {
                let mut data = format!("{}:P-O-,{}", &contract.contractid, &new_owner.1);
                if new_owner.2 {
                    data = format!("{}:P-DO-,{}", &contract.contractid, &new_owner.1);
                }

                write_to_file(
                    format!("./Json/UTXOS/{}.txt", &listing.change_utxo),
                    data.clone(),
                );
            }

            let _ = update_list_utxos(listing.clone(), contract.clone(), true, &result.0[0]);
        }
    } else {
        record_failed_transaction(txid, "handle_list_payload_failed");
    }
}

fn update_list_utxos(
    listing: Listing,
    contract: SCL01Contract,
    pending: bool,
    order_id: &String,
) -> Result<i32, String> {
    let mut highest_bid = 0;
    let mut lowest_bid = 0;
    let mut num_bids = 0;
    let bids = match contract.bids {
        Some(b) => b,
        None => HashMap::new(),
    };

    for (_, b) in bids {
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
    if pending {
        data = format!(
            "{}:P-L-,{},{},{},{},{}",
            &contract.contractid,
            &listing.list_amt,
            &listing.price,
            num_bids,
            highest_bid,
            lowest_bid
        );
        write_to_file(
            format!("./Json/UTXOS/{}.txt", &listing.list_utxo),
            data.clone(),
        );
    } else {
        data = format!(
            "{}:L-,{},{},{},{},{}",
            &contract.contractid,
            &listing.list_amt,
            &listing.price,
            num_bids,
            highest_bid,
            lowest_bid
        );
        write_to_file(
            format!("./Json/UTXOS/{}.txt", &listing.list_utxo),
            data.clone(),
        );
    }
    return Ok(0);
}

pub async fn perform_bid(
    txid: &str,
    command: &str,
    payload: &str,
    trade_txs: &Vec<TradeTx>,
    pending: bool,
) {

    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let listings = match contract.listings.clone() {
        Some(listings) => listings,
        None => {
            record_failed_transaction(txid, "no_listings_found");
            return;
        }
    };


    let words: Vec<&str> = command.split("BID").collect();
    if words.len() < 2 {
        record_failed_transaction(txid, "split_bid_failed");
        return;
    }


    let bid_split: Vec<&str> = words[1].split("],").collect();
    if bid_split.len() < 1 {
        record_failed_transaction(txid, "bid_split_failed");
        return;
    }

    let mut bids: Vec<Bid> = Vec::new();
    let mut bidding_ids: Vec<String> = Vec::new();
    let mut order_id_split = String::new();
    for split in bid_split {
        let bid_info: Vec<&str> = split.split(",").collect();
        if bid_info.len() < 4 {
            record_failed_transaction(txid, "bid_info_split_failed");
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


        let listing = match listings.get(&order_id_split) {
            Some(listing) => listing,
            None => {
                record_failed_transaction(txid, "listing_not_found");
                continue;
            }
        };

        if accept_tx == "".to_string() || fulfil_tx == "".to_string() {
            continue;
        }

        let amount_split = replace_payload_special_characters(&bid_info[1].to_string());

        let amt = match amount_split.parse::<u64>() {
            Ok(amt) => amt,
            Err(_) => {
                record_failed_transaction(txid, "amount_parse_failed");
                continue;
            }
        };

        let price_split = replace_payload_special_characters(&bid_info[2].to_string());

        let price = match price_split.parse::<u64>() {
            Ok(amt) => amt,
            Err(_) => {
                record_failed_transaction(txid, "price_parse_failed");
                continue;
            }
        };

        let mut res_utxo_str = replace_payload_special_characters(&bid_info[3].to_string());
        res_utxo_str = res_utxo_str.replace("TXID", txid);

        let txid = match get_txid_from_hash(&fulfil_tx) {
            Ok(txid) => txid,
            Err(_) => {
                record_failed_transaction(txid, "get_txid_from_hash_failed");
                continue;
            }
        };


        let tx_bytes = match decode(&fulfil_tx) {
            Ok(tx_bytes) => tx_bytes,
            Err(_) => {
                record_failed_transaction(&txid, "decode_fulfil_tx_failed");
                continue;
            }
        };


        let transaction: Transaction = match deserialize(&tx_bytes) {
            Ok(transaction) => transaction,
            Err(_) => {
                record_failed_transaction(&txid, "deserialize_tx_bytes_failed");
                continue;
            }
        };


        let rec_add: Address = match listing.rec_addr.parse::<Address>() {
            Ok(a) => a,
            Err(_) => {
                record_failed_transaction(&txid, "parse_rec_addr_failed");
                continue;
            }
        };

        let mut total_value = 0;
        for output in transaction.output {
            if output.script_pubkey.to_string() == rec_add.script_pubkey().to_string() {
                total_value += output.value;
            }
        }
        let payed_amt = (amt as u128 * price as u128) / 10u64.pow(contract.decimals as u32) as u128;
        if total_value < payed_amt as u64 {
            continue;
        }

        bidding_ids.push(txid.clone());

        let fullfilment_utxos = match get_utxos_from_hash(&fulfil_tx) {
            Ok(fullfilment_utxos) => fullfilment_utxos,
            Err(_) => {
                record_failed_transaction(&txid, "get_utxos_from_hash_failed");
                continue;
            }
        };


        if fullfilment_utxos.len() == 0 {
            record_failed_transaction(&txid, "no_fullfilment_utxos");
            continue;
        }

        let bid: Bid = Bid {
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


    let block_height = match get_current_block_height().await {
        Ok(block_height) => block_height,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            0
        }
    };


    match contract.bid(
        &txid.to_string(),
        &payload.to_string(),
        bids.clone(),
        &bidding_ids,
        block_height,
    ) {
        Ok(_) => {}
        Err(_) => {
            record_failed_transaction(txid, "contract_bid_failed");
            return;
        }
    };


    let _ = save_contract(&contract, &payload, &txid, true);


    let default_listings = HashMap::new();
    if !pending {
        let listings = match contract.listings {
            Some(ref p) => p,
            None => &default_listings,
        };

        let l = match listings.get(&order_id_split.clone()) {
            Some(listing) => listing,
            None => {
                record_failed_transaction(txid, "listing_not_found_final");
                return;
            }
        };

        for b in &bids {
            let data = format!(
                "{}:B-,{},{},{},{}",
                &contract.contractid,
                b.bid_amount,
                b.bid_price,
                0.to_string(),
                l.list_utxo.clone()
            );
            write_to_file(format!("./Json/UTXOS/{}.txt", b.reseved_utxo), data.clone());
        }

        let _ = save_contract(&contract, payload, txid, false);
        _ = update_list_utxos(l.clone(), contract.clone(), false, &order_id_split.clone());

    } else {
        let listings = match contract.listings {
            Some(ref p) => p,
            None => &default_listings,
        };

        let l = match listings.get(&order_id_split.clone()) {
            Some(listing) => listing,
            None => {
                record_failed_transaction(txid, "listing_not_found_final_pending");
                return;
            }
        };

        for b in &bids {
            let data = format!(
                "{}:P-B-,{},{},{},{}",
                &contract.contractid,
                b.bid_amount,
                b.bid_price,
                0.to_string(),
                l.list_utxo.clone()
            );
            write_to_file(format!("./Json/UTXOS/{}.txt", b.reseved_utxo), data.clone());
        }
    }
}

pub async fn perform_accept_bid(txid: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let bids_available = match contract.bids.clone() {
        Some(bids_available) => bids_available,
        None => {
            record_failed_transaction(txid, "no_bids_available");
            return;
        }
    };

    let listings_available = match contract.listings.clone() {
        Some(listings_available) => listings_available,
        None => {
            record_failed_transaction(txid, "no_listings_available");
            return;
        }
    };

    let mut listing_utxos: Vec<String> = Vec::new();
    let mut bid_id: String = "".to_string();
    let mut order_id: String = "".to_string();
    for (key, value) in bids_available.clone() {
        let accept_txid = match get_txid_from_hash(&value.accept_tx) {
            Ok(accept_txid) => accept_txid,
            Err(_) => {
                record_failed_transaction(txid, "get_txid_from_hash_failed");
                return;
            }
        };

        if accept_txid == txid {
            bid_id = key;
            order_id = value.order_id;
        }
    }

    if bid_id == "" || order_id == "" {
        record_failed_transaction(txid, "bid_id_or_order_id_empty");
        return;
    }

    listing_utxos.push(listings_available[&order_id].list_utxo.clone());

    if !check_utxo_inputs(&listing_utxos, &txid.to_string()).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    match contract.accept_bid(&txid.to_string(), &payload.to_string(), &bid_id) {
        Ok(_) => {}
        Err(_) => {
            record_failed_transaction(txid, "accept_bid_failed");
            return;
        }
    };

    if pending {
        let fulfill_payload = format!("{{{}:FULFIL_TRADE}}", contract_id);
        let (new_owners, _, _) =
            match contract.fulfil(&bid_id, &fulfill_payload.to_string(), &bid_id) {
                Ok(n) => n,
                Err(_) => return,
            };
        for (key, value) in &new_owners {
            let data = format!("{}:P-O-,{}", &contract.contractid, value);
            write_to_file(format!("./Json/UTXOS/{}.txt", &key), data.clone());
        }
    }

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        let _ = save_contract(&contract, payload, txid, false);
    }
}

pub async fn perform_fulfil_bid(txid: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let listings = match contract.listings.clone() {
        Some(listings) => listings,
        None => {
            record_failed_transaction(txid, "no_listings_available");
            return;
        }
    };

    let bids = match contract.bids.clone() {
        Some(bids) => bids,
        None => {
            record_failed_transaction(txid, "no_bids_available");
            return;
        }
    };

    let fulfillments = match contract.fulfillments.clone() {
        Some(fulfillments) => fulfillments,
        None => {
            record_failed_transaction(txid, "no_fulfillments_available");
            return;
        }
    };

    if !fulfillments.contains_key(txid) {
        record_failed_transaction(txid, "fulfillment_not_found");
        return;
    }

    let order_id = fulfillments[txid].clone();
    let fulfillment = FulfilledSummary {
        bid_price: bids[txid].bid_price.clone(),
        bid_amount: bids[txid].bid_amount.clone(),
        listing_amount: listings[&order_id].list_amt.clone(),
        listing_price: listings[&order_id].price.clone(),
    };

    let (new_owners, bids, listing) =
        match contract.fulfil(&txid.to_string(), &payload.to_string(), &txid.to_string()) {
            Ok(n) => n,
            Err(_) => {
                record_failed_transaction(txid, "fulfil_failed");
                return;
            }
        };

    let _ = save_contract(&contract, payload, &txid, true);

    if !pending {
        for (key, value) in &new_owners {
            let data = format!("{}:O-,{}", &contract.contractid, value);
            write_to_file(format!("./Json/UTXOS/{}.txt", &key), data.clone());
        }

        let _ = save_contract(&contract, payload, &txid, false);
        for s in &bids {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            };
        }

        let listing_file_path = format!("./Json/UTXOS/{}.txt", listing);
        match fs::remove_file(listing_file_path) {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut interactions = match read_contract_interactions(&contract_id) {
            Ok(interactions) => interactions,
            Err(_) => return,
        };

        interactions.fulfillment_summaries.push(fulfillment);
        match save_contract_interactions(&interactions, &contract_id) {
            Ok(_) => interactions,
            Err(_) => return,
        };
    } else {
        for (key, value) in &new_owners {
            let data = format!("{}:P-O-,{}", &contract.contractid, value);
            write_to_file(format!("./Json/UTXOS/{}.txt", &key), data.clone());
        }
    }
}

pub async fn perform_drip_start(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_drip_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_drip_payload_failed");
            return;
        }
    };

    if !check_utxo_inputs(&results.0, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let current_block_height = match get_current_block_height().await {
        Ok(current_block_height) => current_block_height,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            return;
        }
    };

    let new_owners = match contract.start_drip(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        &results.2,
        current_block_height as u64,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "start_drip_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        for s in &results.0 {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        for (key, value) in new_owners.0.clone() {
            let data = format!("{}:DO-,{}", &contract.contractid, value);
            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {}
            };
        }
        let data = format!("{}:O-,{}", &contract.contractid, &new_owners.1 .1);
        match fs::write(
            format!("./Json/UTXOS/{}.txt", &new_owners.1 .0.clone()),
            data.clone(),
        ) {
            Ok(_) => {}
            Err(_) => return,
        };

        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let data = format!("{}:P-O-,{}", &contract.contractid, &new_owners.1 .1);
        match fs::write(
            format!("./Json/UTXOS/{}.txt", &new_owners.1 .0.clone()),
            data.clone(),
        ) {
            Ok(_) => {}
            Err(_) => {}
        };

        for (key, value) in new_owners.0 {
            let data = format!("{}:P-DO-,{}", &contract.contractid, value);
            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {}
            };
        }
    }
}

pub async fn perform_create_diminishing_airdrop(
    txid: &str,
    command: &str,
    payload: &str,
    pending: bool,
) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_create_diminishing_airdrop_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_create_diminishing_airdrop_payload_failed");
            return;
        }
    };

    if !check_utxo_inputs(&results.0, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let current_block_height = match get_current_block_height().await {
        Ok(current_block_height) => current_block_height as u64,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            return;
        }
    };

    let new_owners = match contract.create_dim_airdrop(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        &results.2,
        &results.3,
        &results.4,
        &results.5,
        &results.6,
        &results.7,
        current_block_height,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "create_dim_airdrop_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        for s in &results.0 {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        let mut data = format!("{}:O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let mut data = format!("{}:P-O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
    }
}

pub async fn perform_claim_diminishing_airdrop(
    txid: &str,
    command: &str,
    payload: &str,
    pending: bool,
    esplora: String,
) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let results = match handle_claim_diminishing_airdrop_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_claim_diminishing_airdrop_payload_failed");
            return;
        }
    };

    let contract_pending = match read_contract(contract_id.as_str(), true) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_pending_failed");
            return;
        }
    };

    let dims = match contract_pending.diminishing_airdrops.clone() {
        Some(dims) => dims,
        None => {
            record_failed_transaction(txid, "no_diminishing_airdrops");
            return;
        }
    };

    let dim: DimAirdrop = match dims.get(&results.0) {
        Some(dim) => dim.clone(),
        None => {
            record_failed_transaction(txid, "dim_airdrop_not_found");
            return;
        }
    };

    let mut donater_pub_address: String = String::new();

    if dim.single_drop {
        let url: String = esplora.to_string() + "tx/" + &txid;
        let response = match handle_get_request(url).await {
            Some(response) => response,
            None => {
                record_failed_transaction(txid, "handle_get_request_failed");
                return;
            }
        };

        let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
            Ok(tx_info) => tx_info,
            Err(_) => {
                record_failed_transaction(txid, "serde_json_parse_txinfo_failed");
                return;
            }
        };

        let vin = match tx_info.vin {
            Some(vin) => vin,
            None => {
                record_failed_transaction(txid, "no_vin_in_txinfo");
                return;
            }
        };

        if vin.len() == 0 {
            record_failed_transaction(txid, "vin_empty");
            return;
        }

        let prev_outputs = match &vin[0].prevout {
            Some(prev) => prev,
            None => {
                record_failed_transaction(txid, "no_prevout_in_vin");
                return;
            }
        };

        donater_pub_address = match &prev_outputs.scriptpubkey_address {
            Some(donater_pub_address) => donater_pub_address.clone(),
            None => {
                record_failed_transaction(txid, "no_scriptpubkey_address_in_prevout");
                return;
            }
        };

        if dim.claimers.contains_key(&donater_pub_address) {
            record_failed_transaction(txid, "claimer_already_exists");
            return;
        }
    }

    let p_c: HashMap<String, u64> = match contract_pending.pending_claims.clone() {
        Some(p_c) => p_c,
        None => HashMap::new(),
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    contract.pending_claims = Some(p_c);
    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let new_owners = match contract.claim_dim_airdrop(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        pending,
        &donater_pub_address,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "claim_dim_airdrop_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        let mut data = format!("{}:O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());

        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let mut data = format!("{}:P-C-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DC-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
    }
}

pub async fn perform_create_dge(txid: &str, command: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_create_dge_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_create_dge_payload_failed");
            return;
        }
    };

    if !check_utxo_inputs(&results.0, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let dge: DGE = DGE {
        pool_amount: results.1,
        sats_rate: results.2,
        max_drop: results.3,
        current_amount_dropped: 0,
        donations_address: results.5,
        drip_duration: results.4,
        donaters: HashMap::new(),
        single_drop: results.7,
    };

    let current_block_height = match get_current_block_height().await {
        Ok(current_block_height) => current_block_height as u64,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            return;
        }
    };

    let new_owners = match contract.create_dge(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        dge,
        &results.6,
        current_block_height,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "create_dge_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);

    if !pending {
        for s in &results.0 {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        let mut data = format!("{}:O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:DO-,{}", &contract.contractid, new_owners.1);
        }

        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let mut data = format!("{}:P-O-,{}", &contract.contractid, new_owners.1);
        if new_owners.2 {
            data = format!("{}:P-DO-,{}", &contract.contractid, new_owners.1);
        }
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
    }
}

pub async fn perform_claim_dge(
    txid: &str,
    command: &str,
    payload: &str,
    pending: bool,
    esplora: String,
) {
    let contract_id = match extract_contract_id(command) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let results = match handle_claim_dge_payload(txid, command) {
        Ok(results) => results,
        Err(_) => {
            record_failed_transaction(txid, "handle_claim_dge_payload_failed");
            return;
        }
    };

    let dges = match contract.dges.clone() {
        Some(dges) => dges,
        None => {
            record_failed_transaction(txid, "no_dges");
            return;
        }
    };

    let dge: DGE = match dges.get(&results.0) {
        Some(dge) => dge.clone(),
        None => {
            record_failed_transaction(txid, "dge_not_found");
            return;
        }
    };

    let url: String = esplora.to_string() + "tx/" + &txid;
    let response = match handle_get_request(url).await {
        Some(response) => response,
        None => {
            record_failed_transaction(txid, "handle_get_request_failed");
            return;
        }
    };

    let tx_info: TxInfo = match serde_json::from_str::<TxInfo>(&response) {
        Ok(tx_info) => tx_info,
        Err(_) => {
            record_failed_transaction(txid, "serde_json_parse_txinfo_failed");
            return;
        }
    };

    let vin = match tx_info.vin {
        Some(vin) => vin,
        None => {
            record_failed_transaction(txid, "no_vin_in_txinfo");
            return;
        }
    };

    if vin.len() == 0 {
        record_failed_transaction(txid, "vin_empty");
        return;
    }

    let prev_outputs = match &vin[0].prevout {
        Some(prev) => prev,
        None => {
            record_failed_transaction(txid, "no_prevout_in_vin");
            return;
        }
    };

    let donater_pub_address = match &prev_outputs.scriptpubkey_address {
        Some(donater_pub_address) => donater_pub_address.clone(),
        None => {
            record_failed_transaction(txid, "no_scriptpubkey_address_in_prevout");
            return;
        }
    };

    let vout = match tx_info.vout {
        Some(vout) => vout,
        None => {
            record_failed_transaction(txid, "no_vout_in_txinfo");
            return;
        }
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
        record_failed_transaction(txid, "donation_amount_zero");
        return;
    }

    if dge.single_drop && dge.donaters.contains_key(&donater_pub_address) {
        record_failed_transaction(txid, "donater_already_exists");
        return;
    }

    let current_block = match get_current_block_height().await {
        Ok(current_block) => current_block as u64,
        Err(_) => {
            record_failed_transaction(txid, "get_current_block_height_failed");
            return;
        }
    };

    let new_owners = match contract.claim_dge(
        &txid.to_string(),
        &payload.to_string(),
        &results.0,
        &results.1,
        &donater_pub_address,
        donation_amout,
        current_block,
    ) {
        Ok(res) => res,
        Err(_) => {
            record_failed_transaction(txid, "claim_dge_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, txid, true);
    if !pending {
        let data = format!("{}:DO-,{}", &contract.contractid, new_owners.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());

        let _ = save_contract(&contract, payload, txid, false);
    } else {
        let data = format!("{}:P-DO-,{}", &contract.contractid, new_owners.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &new_owners.0), data.clone());
    }
}

pub fn perform_drips(contract_id: String, block_height: u64, pending: bool) {
    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let new_owners = match contract.drip(block_height as u64) {
        Ok(res) => res,
        Err(_) => return,
    };

    if new_owners.len() == 0 {
        return;
    }

    let _ = save_contract(&contract, "", "", true);

    if !pending {
        for (key, value, drip) in new_owners.clone() {
            let mut data = format!("{}:O-,{}", &contract.contractid, value);
            if drip {
                data = format!("{}:DO-,{}", &contract.contractid, value);
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {}
            };
        }

        let _ = save_contract(&contract, "", "", false);
    } else {
        for (key, value, drip) in new_owners.clone() {
            let mut data = format!("{}:P-O-,{}", &contract.contractid, value);
            if drip {
                data = format!("{}:P-DO-,{}", &contract.contractid, value);
            }
            match fs::write(format!("./Json/UTXOS/{}.txt", &key), data.clone()) {
                Ok(_) => {}
                Err(_) => {}
            };
        }
    }
}

pub async fn perform_listing_cancel(txid: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let words: Vec<&str> = payload.split("CANCELLISTING").collect();
    if words.len() < 2 {
        record_failed_transaction(txid, "malformed_cancel_listing_command");
        return;
    }

    let listing_utxo = replace_payload_special_characters(&words[1].to_string());
    let utxos: Vec<String> = vec![listing_utxo.clone()];
    if !check_utxo_inputs(&utxos, &txid).await {
        record_failed_transaction(txid, "check_utxo_inputs_failed");
        return;
    }

    let file_path = format!("./Json/UTXOS/{}.txt", listing_utxo);
    // Attempt to remove the file
    match fs::remove_file(file_path) {
        Ok(_) => {}
        Err(_) => {}
    }

    let (owner, bids) = match contract.cancel_listing(
        &txid.to_string(),
        &listing_utxo.to_string(),
        payload.to_string(),
    ) {
        Ok(owner) => owner,
        Err(_) => {
            record_failed_transaction(txid, "cancel_listing_failed");
            return;
        }
    };

    let _ = save_contract(&contract, payload, &txid, pending);
    if !pending {
        let _ = save_contract(&contract, payload, &txid, false);
        let data = format!("{}:O-,{}", &contract.contractid, owner.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", &owner.0), data.clone());
        for s in &bids {
            let file_path = format!("./Json/UTXOS/{}.txt", s);
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    } else {
        let data = format!("{}:P-O-,{}", &contract.contractid, owner.1);
        write_to_file(format!("./Json/UTXOS/{}.txt", owner.0), data.clone());
    }
}

pub async fn perform_bid_cancel(txid: &str, payload: &str, pending: bool) {
    let contract_id = match extract_contract_id(payload) {
        Ok(contract_id) => contract_id,
        Err(_) => {
            record_failed_transaction(txid, "extract_contract_id_failed");
            return;
        }
    };

    let mut contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => {
            record_failed_transaction(txid, "read_contract_failed");
            return;
        }
    };

    if contract.payloads.iter().any(|(tx, _)| tx == txid) {
        record_failed_transaction(txid, "duplicate_txid_in_payloads");
        return;
    }

    let words: Vec<&str> = payload.split("CANCELBID").collect();
    if words.len() < 2 {
        return;
    }

    let bidding_utxo = replace_payload_special_characters(&words[1].to_string());
    let utxos: Vec<String> = vec![bidding_utxo.clone()];
    if !check_utxo_inputs(&utxos, &txid).await {
        return;
    }

    let file_path = format!("./Json/UTXOS/{}.txt", bidding_utxo);
    // Attempt to remove the file
    match fs::remove_file(file_path) {
        Ok(_) => {}
        Err(_) => {}
    }

    match contract.cancel_bid(
        &txid.to_string(),
        &bidding_utxo.to_string(),
        payload.to_string(),
    ) {
        Ok(_) => {}
        Err(_) => return,
    }

    let _ = save_contract(&contract, payload, &txid, pending);
    if !pending {
        let _ = save_contract(&contract, payload, &txid, false);
    }
}

pub fn handle_create_diminishing_airdrop_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, u64, u64, u64, u64, u64, String, bool), String> {
    let words: Vec<&str> = payload.split("DIMAIRDROP").collect();
    if words.len() < 2 {
        return Err("Invalid dim airdrop payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 2 {
        return Err("Invalid drip payload".to_string());
    }

    let sender_strs: Vec<&str> = sendsplit[0].split(",").collect();
    let mut senders: Vec<String> = Vec::new();
    for sendi in sender_strs {
        let sender = replace_payload_special_characters(&sendi.to_string());
        senders.push(sender);
    }
    let split: Vec<&str> = sendsplit[1].split(",").collect();
    if split.len() < 7 {
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
    return Ok((
        senders,
        pool,
        step_amount,
        step_period,
        max_airdrop,
        min_airdrop,
        change,
        single_drop,
    ));
}

pub fn handle_claim_diminishing_airdrop_payload(
    txid: &str,
    payload: &str,
) -> Result<(String, String), String> {
    let words: Vec<&str> = payload.split("CLAIM_DIMAIRDROP").collect();
    if words.len() < 2 {
        return Err("Invalid dim airdrop claim payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 2 {
        return Err("Invalid dim airdrop claim payload".to_string());
    }

    let claim_id = replace_payload_special_characters(&sendsplit[0].to_string());
    let reciever_split = replace_payload_special_characters(&sendsplit[1].to_string());
    let reciever = reciever_split.replace("TXID", txid);
    return Ok((claim_id, reciever));
}

pub fn handle_claim_dge_payload(txid: &str, payload: &str) -> Result<(String, String), String> {
    let words: Vec<&str> = payload.split("CLAIM_DGE").collect();
    if words.len() < 2 {
        return Err("Invalid dge claim payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 2 {
        return Err("Invalid dge claim payload".to_string());
    }

    let claim_id = replace_payload_special_characters(&sendsplit[0].to_string());
    let reciever_split = replace_payload_special_characters(&sendsplit[1].to_string());
    let reciever = reciever_split.replace("TXID", txid);
    return Ok((claim_id, reciever));
}

pub fn handle_create_dge_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, u64, u64, u64, u64, String, String, bool), String> {
    let words: Vec<&str> = payload.split("DGE").collect();
    if words.len() < 2 {
        return Err("Invalid dge creation payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 2 {
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
    return Ok((
        senders,
        pool,
        sats_rate,
        max_drop,
        drip_duration,
        address_split,
        change,
        single_drop,
    ));
}

pub fn read_contract(contract_id: &str, pending: bool) -> Result<SCL01Contract, String> {
    let mut path = "./Json/Contracts/".to_string() + "/" + contract_id + "/state.txt";
    if pending {
        path = "./Json/Contracts/".to_string() + "/" + contract_id + "/pending.txt";
    }
    match read_from_file(path) {
        Some(contract_obj) => {
            let parsed_data: Result<SCL01Contract, serde_json::Error> =
                serde_json::from_str(&contract_obj);
            match parsed_data {
                Ok(data) => return Ok(data),
                Err(_) => return Err("Failed to deserialize contract".to_string()),
            }
        }
        None => return Err("Could not find contract".to_string()),
    }
}

pub fn save_contract(
    contract: &SCL01Contract,
    _payload: &str,
    _txid: &str,
    pending: bool,
) -> Result<String, String> {
    let path: String;
    if !pending {
        path = format!(
            "{}/{}/state.txt",
            "./Json/Contracts/",
            contract.contractid.to_string()
        );
    } else {
        path = format!(
            "{}/{}/pending.txt",
            "./Json/Contracts/", contract.contractid
        );
    }

    match serde_json::to_string(&contract) {
        Ok(state_string) => write_to_file(path, state_string),
        Err(_) => return Err("Failed to save updated contract".to_string()),
    };

    return Ok("Success".to_string());
}

pub fn handle_mint_payload(
    payload: &str,
    txid: &str,
) -> Result<(String, String, u64, u64), String> {
    let mut mint_strings: Vec<String> = Vec::new();
    let mut mint_values: Vec<u64> = Vec::new();
    let re = match Regex::new(r"\[([^,]+),([^,]+),([^,]+),([^]]+)]") {
        Ok(re) => re,
        Err(_) => return Err("Not mint valid payload".to_string()),
    };

    if let Some(captures) = re.captures(payload) {
        let ticker = match captures.get(1) {
            Some(ticker) => ticker.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let max_supply_str = match captures.get(2) {
            Some(max_supply_str) => max_supply_str.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let decimals_str = match captures.get(3) {
            Some(decimals_str) => decimals_str.as_str(),
            None => return Err("Not mint valid payload".to_string()),
        };

        let txid_n = match captures.get(4) {
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
        let max_supply = match max_supply_str.parse() {
            Ok(max_supply) => max_supply,
            Err(_) => return Err("Not mint valid payload".to_string()),
        };

        let decimals = match decimals_str.parse() {
            Ok(decimals) => decimals,
            Err(_) => return Err("Not mint valid payload".to_string()),
        };

        mint_strings.push(ticker.to_string());
        mint_strings.push(t_n.to_string());
        mint_values.push(max_supply);
        mint_values.push(decimals);
        return Ok((ticker.to_string(), t_n, max_supply, decimals));
    }
    return Err("Not mint valid payload".to_string());
}

pub fn handle_mint_rtm_payload(
    payload: &str,
    txid: &str,
) -> Result<(String, u64, HashMap<String, u64>), String> {
    let words: Vec<&str> = payload.split("SCL03:").collect();
    if words.len() < 2 {
        return Err("Invalid transfer payload".to_string());
    }

    let mint_split: Vec<&str> = words[1].split(",").collect();
    if mint_split.len() < 2 {
        return Err("Invalid mint rtm payload".to_string());
    }

    let ticker = replace_payload_special_characters(&mint_split[0].to_string());
    let decimal_split = replace_payload_special_characters(&mint_split[1].to_string());

    // Parse strings to numeric types
    let decimals = match decimal_split.parse() {
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
        if data.len() < 2 {
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

pub fn handle_rtm_payload(
    txid: &str,
    payload: &str,
) -> Result<(String, String, String, u64), String> {
    let words: Vec<&str> = payload.split("RIGHTTOMINT").collect();
    if words.len() < 2 {
        return Err("Invalid rtm payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split(",").collect();
    if sendsplit.len() < 4 {
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

pub fn handle_transfer_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, Vec<(String, u64)>, String), String> {
    let words: Vec<&str> = payload.split("TRANSFER").collect();
    if words.len() < 2 {
        return Err("Invalid transfer payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],[").collect();
    if sendsplit.len() < 2 {
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
        if data.len() < 2 {
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

pub fn handle_drip_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, HashMap<String, (u64, u64)>, String), String> {
    let words: Vec<&str> = payload.split("DRIP").collect();
    if words.len() < 2 {
        return Err("Invalid drip payload".to_string());
    }

    let sendsplit: Vec<&str> = words[1].split("],").collect();
    if sendsplit.len() < 3 {
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

    let mut rec_dict: HashMap<String, (u64, u64)> = HashMap::new();
    for reci in rec_split {
        let rec_str = reci.replace("TXID", txid);
        let recievers = replace_payload_special_characters(&rec_str.to_string());
        let data: Vec<&str> = recievers.split("(").collect();
        if data.len() < 2 {
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

pub fn handle_burn_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, u64, String), String> {
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
    for s in &burners {
        let file_path = format!("./Json/UTXOS/{}.txt", s);
        // Attempt to remove the file
        match fs::remove_file(file_path) {
            Ok(_) => {}
            Err(_) => {}
        }
    }
    return Ok((burners, amt, change_str));
}

pub fn handle_list_payload(
    txid: &str,
    payload: &str,
) -> Result<(Vec<String>, String, String, String, u64, u64), String> {
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

    return Ok((
        listings_senders,
        change_str,
        listing_utxo_str,
        pay_address_str,
        listing_amt,
        sell_price,
    ));
}

pub fn handle_bid_payload(
    txid: &str,
    payload: &str,
) -> Result<Vec<(String, u64, u64, String)>, String> {
    let words: Vec<&str> = payload.split("BID").collect();
    let bid_split: Vec<&str> = words[1].split("],").collect();
    if bid_split.len() < 1 {
        return Err("Invalid Bid payload. Sell price invalid".to_string());
    }

    let mut bid_results: Vec<(String, u64, u64, String)> = Vec::new();
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

pub fn handle_payload_extra_trade_info(payload: &str) -> Result<(String, u64, u64), String> {
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
                        let file_path = format!("./Json/Contracts/{}/state.txt", &folder_name);
                        let user_data_str = match fs::read_to_string(&file_path) {
                            Ok(user_data_str) => user_data_str,
                            Err(_) => continue,
                        };

                        // Deserialize user data from JSON
                        let mut user_data: SCL01Contract =
                            match serde_json::from_str(&user_data_str) {
                                Ok(user_data) => user_data,
                                Err(_) => continue,
                            };

                        let airdrop_amount = match user_data.airdrop_amount {
                            Some(airdrop_amount) => airdrop_amount,
                            None => continue,
                        };

                        let current_airdrops = match user_data.current_airdrops {
                            Some(current_airdrops) => current_airdrops,
                            None => continue,
                        };

                        let total_airdrops = match user_data.total_airdrops {
                            Some(total_airdrops) => total_airdrops,
                            None => continue,
                        };

                        user_data.max_supply = Some(total_airdrops * airdrop_amount);
                        user_data.supply = current_airdrops * airdrop_amount;

                        let serialised_user_data = match serde_json::to_string(&user_data) {
                            Ok(serialised_user_data) => serialised_user_data,
                            Err(_) => continue,
                        };

                        write_to_file(
                            format!("./Json/Contracts/{}/state.txt", &folder_name),
                            serialised_user_data.clone(),
                        );
                        write_to_file(
                            format!("./Json/Contracts/{}/pending.txt", &folder_name),
                            serialised_user_data,
                        );
                    }
                }
            }
        }
    }
}

// Liquidity Pools
pub fn perform_minting_scl04(txid: &str, payload: &str) {
    match read_contract(txid, false) {
        Ok(_) => return,
        Err(_) => {}
    };

    let words: Vec<&str> = payload.split("SCL04:").collect();
    if words.len() < 2 {
        println!("Invalid liquidity pool payload");
        return;
    }

    let mint_split: Vec<&str> = words[1].split(",").collect();
    if mint_split.len() < 5 {
        println!("Invalid liquidity pool payload");
        return;
    }

    let ticker = replace_payload_special_characters(&mint_split[0].to_string());
    let contract_id_1 = replace_payload_special_characters(&mint_split[1].to_string());
    let contract_id_2 = replace_payload_special_characters(&mint_split[2].to_string());
    let ratio_split = replace_payload_special_characters(&mint_split[3].to_string());
    let fee_split = replace_payload_special_characters(&mint_split[4].to_string());

    // Parse strings to numeric types
    let ratio = match ratio_split.parse::<f64>() {
        Ok(ratio) => ratio,
        Err(_) => {
            println!("Not mint valid payload");
            return;
        }
    };

    let fee = match fee_split.parse::<f32>() {
        Ok(fee) => fee,
        Err(_) => {
            println!("Not mint valid payload");
            return;
        }
    };

    if contract_id_1 == contract_id_2 {
        return;
    }

    let contract_1 = match read_contract(&contract_id_1, false) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let contract_2 = match read_contract(&contract_id_2, false) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract_1.decimals != contract_2.decimals {
        return;
    }

    let mut payloads: HashMap<String, String> = HashMap::new();
    payloads.insert(txid.to_string(), payload.to_string());
    let pools: LiquidityPool = LiquidityPool {
        contract_id_1: contract_id_1,
        contract_id_2: contract_id_2,
        pool_1: 0,
        pool_2: 0,
        fee: fee,
        k: 0,
        liquidity_ratio: ratio,
        swaps: HashMap::new(),
        liquidations: HashMap::new(),
    };

    let new_contract = SCL01Contract {
        ticker: ticker,
        contractid: txid.to_string(),
        supply: 0,
        decimals: contract_1.decimals,
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
        right_to_mint: None,
        max_supply: None,
        liquidated_tokens: None,
        liquidity_pool: Some(pools),
        token_data: None,
    };

    match serde_json::to_string(&new_contract) {
        Ok(s) => {
            write_contract_directory(
                format!("./Json/Contracts/{}/state.txt", &new_contract.contractid),
                s.clone(),
                new_contract.contractid.as_str(),
            );
            write_contract_directory(
                format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),
                s.clone(),
                new_contract.contractid.as_str(),
            );
            let path =
                "./Json/Contracts/".to_string() + "/" + &new_contract.contractid + "/header.txt";
            let config = match read_server_config() {
                Ok(config) => config,
                Err(_) => Config::default(),
            };

            let url = match config.url {
                Some(url) => url,
                None => "https://scl.darkfusion.tech/".to_owned(),
            };

            let import = ContractImport {
                contract_id: new_contract.contractid.clone(),
                ticker: new_contract.ticker,
                rest_url: url.to_string(),
                contract_type: "SCL04".to_string(),
                decimals: new_contract.decimals,
            };

            let result = match serde_json::to_string(&import) {
                Ok(result) => result,
                Err(_) => return,
            };

            write_to_file(path, result);

            let mut lookup = match read_server_lookup() {
                Ok(lookup) => lookup,
                Err(_) => Lookups::default(),
            };

            lookup.lps.push(new_contract.contractid);
            let _ = save_server_lookup(lookup);
        }
        Err(_) => {}
    };
}

pub fn perform_minting_scl05(txid: &str, payload: &str) {
    match read_contract(txid, false) {
        Ok(_) => return,
        Err(_) => {}
    };

    let words: Vec<&str> = payload.split("SCL05:").collect();
    if words.len() < 2 {
        println!("Invalid Non-Fungible Token payload");
        return;
    }

    let mint_split: Vec<&str> = words[1].split(",").collect();
    if mint_split.len() < 3 {
        println!("Invalid Non-Fungible Token payload");
        return;
    }

    let ticker = replace_payload_special_characters(&mint_split[0].to_string());
    let utxo_rec = replace_payload_special_characters(&mint_split[1].to_string());
    let base_64 = replace_payload_special_characters(&mint_split[2].to_string());
    let mut payloads: HashMap<String, String> = HashMap::new();
    payloads.insert(txid.to_string(), payload.to_string());
    let mut owners: HashMap<String, u64> = HashMap::new();
    owners.insert(utxo_rec, 1);
    let new_contract = SCL01Contract {
        ticker,
        contractid: txid.to_string(),
        supply: 1,
        decimals: 0,
        owners,
        payloads,
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
        max_supply: None,
        liquidated_tokens: None,
        liquidity_pool: None,
        token_data: Some(base_64),
    };

    match serde_json::to_string(&new_contract) {
        Ok(s) => {
            write_contract_directory(
                format!("./Json/Contracts/{}/state.txt", &new_contract.contractid),
                s.clone(),
                new_contract.contractid.as_str(),
            );
            write_contract_directory(
                format!("./Json/Contracts/{}/pending.txt", &new_contract.contractid),
                s.clone(),
                new_contract.contractid.as_str(),
            );
            let config = match read_server_config() {
                Ok(config) => config,
                Err(_) => Config::default(),
            };

            let url = match config.url {
                Some(url) => url,
                None => "https://scl.darkfusion.tech/".to_owned(),
            };

            let path =
                "./Json/Contracts/".to_string() + "/" + &new_contract.contractid + "/header.txt";
            let import = ContractImport {
                contract_id: new_contract.contractid,
                ticker: new_contract.ticker,
                rest_url: url,
                contract_type: "SCL05".to_string(),
                decimals: new_contract.decimals,
            };
            let result = match serde_json::to_string(&import) {
                Ok(result) => result,
                Err(_) => return,
            };
            write_to_file(path, result);
        }
        Err(_) => {}
    };
}

pub async fn perform_provide_liquidity(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let captures = match handle_provide_liquidity_payload_lp(&payload) {
        Ok(captures) => captures,
        Err(_) => {
            println!("Failed to parse liquity provision payload");
            return;
        }
    };

    let lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let lp_pool = match lp_contract.liquidity_pool {
        Some(lp_pool) => lp_pool,
        None => return,
    };

    let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
        Ok(inputs) => inputs,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut contract_1 = match read_contract(lp_pool.contract_id_1.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract_1.payloads.contains_key(txid) {
        return;
    }

    let res_1 = match contract_1.provide_liquidity(
        &txid.to_string(),
        &payload.to_string(),
        &input_utxos,
        captures,
        block_height as u64,
        true,
    ) {
        Ok(res) => res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut contract_2 = match read_contract(lp_pool.contract_id_2.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract_2.payloads.contains_key(txid) {
        return;
    }

    let res_2 = match contract_2.provide_liquidity(
        &txid.to_string(),
        &payload.to_string(),
        &input_utxos,
        captures,
        block_height as u64,
        false,
    ) {
        Ok(res) => res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let _ = save_contract(&contract_1, payload, txid, pending);
    let _ = save_contract(&contract_2, payload, txid, pending);
    save_check_utxo_file(
        &contract_2.contractid,
        &res_2.0,
        res_2.1,
        res_2.2,
        pending,
        "O",
    );
    save_check_utxo_file(
        &contract_1.contractid,
        &res_1.0,
        res_1.1,
        res_1.2,
        pending,
        "O",
    );
    if !pending {
        let _ = save_contract(&contract_1, payload, txid, true);
        let _ = save_contract(&contract_2, payload, txid, true);
        for utxo in input_utxos {
            let file_path = format!("./Json/UTXOS/{}.txt", utxo);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}

pub async fn perform_provide_liquidity_lp(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let mut lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if lp_contract.payloads.contains_key(txid) {
        return;
    }

    let lp = match lp_contract.liquidity_pool.clone() {
        Some(lp) => lp,
        None => return,
    };

    let captures = match handle_provide_liquidity_payload_lp(&payload) {
        Ok(captures) => captures,
        Err(_) => {
            println!("Failed to parse liquity provision payload");
            return;
        }
    };

    let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
        Ok(inputs) => inputs,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut contract_1 = match read_contract(&lp.contract_id_1, pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if !contract_1.payloads.contains_key(txid) {
        match contract_1.provide_liquidity(
            &txid.to_string(),
            &payload.to_string(),
            &input_utxos,
            captures,
            block_height as u64,
            true,
        ) {
            Ok(res) => res,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };
    }

    let mut contract_2 = match read_contract(&lp.contract_id_2, pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if !contract_2.payloads.contains_key(txid) {
        let amount: u64 = (captures as f64 * lp.liquidity_ratio as f64) as u64;
        match contract_2.provide_liquidity(
            &txid.to_string(),
            &payload.to_string(),
            &input_utxos,
            amount,
            block_height as u64,
            false,
        ) {
            Ok(res) => res,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };
    }

    let lp_res =
        match lp_contract.provide_liquidity_lp(&txid.to_string(), &payload.to_string(), captures) {
            Ok(lp_res) => lp_res,
            Err(err) => {
                println!("Failed to execute liquity provision: {}", err);
                return;
            }
        };

    let _ = save_contract(&lp_contract, payload, txid, pending);
    let mut balance_type = "O";
    if pending {
        balance_type = "U";
    }

    save_check_utxo_file(
        &lp_contract.contractid,
        &lp_res.0,
        lp_res.1,
        false,
        pending,
        balance_type,
    );
    if !pending {
        let _ = save_contract(&lp_contract, payload, txid, true);
    }
}

pub async fn perform_swap(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let mut lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let liquidity_pool = match lp_contract.liquidity_pool.clone() {
        Some(liquidity_pool) => liquidity_pool,
        None => return,
    };

    let lp_captures = match handle_swap_payload_lp(&payload) {
        Ok(captures) => captures,
        Err(_) => return,
    };

    let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
        Ok(inputs) => inputs,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let reciever_contract_id;
    let claimer_contract_id;
    if lp_captures.0 {
        reciever_contract_id = liquidity_pool.contract_id_2;
        claimer_contract_id = liquidity_pool.contract_id_1;
    } else {
        reciever_contract_id = liquidity_pool.contract_id_1;
        claimer_contract_id = liquidity_pool.contract_id_2;
    }

    let mut claimer_contract = match read_contract(claimer_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let mut reciever_contract = match read_contract(reciever_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let recieving_amount: u64;
    if liquidity_pool.swaps.contains_key(txid) {
        recieving_amount = liquidity_pool.swaps[txid].1;
    } else {
        recieving_amount = match lp_contract.swap_lp(
            &txid.to_string(),
            &payload.to_string(),
            claimer_contract_id,
            lp_captures.1,
            lp_captures.2,
            lp_captures.3,
        ) {
            Ok(lp_res) => lp_res,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };
    }

    let mut reciever_res: (String, u64) = (String::new(), 0);
    if !reciever_contract.payloads.contains_key(txid) {
        reciever_res = match reciever_contract.swap_recieve(
            &txid.to_string(),
            &payload.to_string(),
            recieving_amount,
        ) {
            Ok(res) => res,
            Err(err) => {
                println!("{}", err);
                (String::new(), 0)
            }
        };
    }

    let mut claim_res: (String, u64, bool) = (String::new(), 0, false);
    let mut swap_amount = lp_captures.1;
    if recieving_amount == 0 {
        swap_amount = 0;
    }
    if !claimer_contract.payloads.contains_key(txid) {
        claim_res = match claimer_contract.swap_claim(
            &txid.to_string(),
            &payload.to_string(),
            &input_utxos,
            swap_amount,
            block_height as u64,
        ) {
            Ok(res) => res,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };
    }

    let _ = save_contract(&claimer_contract, payload, txid, pending);
    let _ = save_contract(&reciever_contract, payload, txid, pending);
    let mut balance_type = "O";
    if pending {
        balance_type = "U";
    }

    save_check_utxo_file(
        &reciever_contract.contractid,
        &reciever_res.0,
        reciever_res.1,
        false,
        pending,
        balance_type,
    );
    save_check_utxo_file(
        &claimer_contract.contractid,
        &claim_res.0,
        claim_res.1,
        claim_res.2,
        pending,
        balance_type,
    );
    if !pending {
        let _ = save_contract(&claimer_contract, payload, txid, true);
        let _ = save_contract(&reciever_contract, payload, txid, true);

        for utxo in input_utxos {
            let file_path = format!("./Json/UTXOS/{}.txt", utxo);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}

pub async fn perform_swap_lp(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let mut lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if lp_contract.payloads.contains_key(txid) {
        return;
    }

    let lp_pool = match lp_contract.liquidity_pool.clone() {
        Some(lp_pool) => lp_pool,
        None => return,
    };

    let lp_captures = match handle_swap_payload_lp(&payload) {
        Ok(captures) => captures,
        Err(_) => return,
    };

    let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
        Ok(inputs) => inputs,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let contract_id;
    if lp_captures.0 {
        contract_id = lp_pool.contract_id_1;
    } else {
        contract_id = lp_pool.contract_id_2;
    }

    let mut sender_contract = match read_contract(contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if sender_contract.payloads.contains_key(txid) {
        return;
    }

    match sender_contract.swap_claim(
        &txid.to_string(),
        &payload.to_string(),
        &input_utxos,
        lp_captures.1,
        block_height as u64,
    ) {
        Ok(res) => res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    _ = match lp_contract.swap_lp(
        &txid.to_string(),
        &payload.to_string(),
        contract_id,
        lp_captures.1,
        lp_captures.2,
        lp_captures.3,
    ) {
        Ok(lp_res) => lp_res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    if !pending {
        let _ = save_contract(&lp_contract, payload, txid, false);
    } else {
        let _ = save_contract(&lp_contract, payload, txid, true);
    }
}

pub async fn perform_liquidate_position(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let mut lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    let liquidity_pool = match lp_contract.liquidity_pool.clone() {
        Some(liquidity_pool) => liquidity_pool,
        None => return,
    };

    let lp_res: (u64, u64, String, u64, bool);
    if !liquidity_pool.liquidations.contains_key(txid) {
        let captures = match handle_liquidatation_payload_lp(&payload) {
            Ok(captures) => captures,
            Err(_) => return,
        };

        let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
            Ok(inputs) => inputs,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

        lp_res = match lp_contract.liquidate_postion_lp(
            &txid.to_string(),
            &payload.to_string(),
            &input_utxos,
            captures,
            block_height as u64,
        ) {
            Ok(lp_res) => lp_res,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };
    } else {
        lp_res = (
            liquidity_pool.liquidations[txid].0,
            liquidity_pool.liquidations[txid].1,
            format!("{}:0", txid),
            0,
            false,
        )
    }

    let mut contract_1 = match read_contract(liquidity_pool.contract_id_1.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract_1.payloads.contains_key(txid) {
        return;
    }

    let res_1 = match contract_1.liquidate_position(
        &txid.to_string(),
        &payload.to_string(),
        lp_res.0,
        true,
    ) {
        Ok(res) => res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut contract_2 = match read_contract(liquidity_pool.contract_id_2.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if contract_2.payloads.contains_key(txid) {
        return;
    }

    let res_2 = match contract_2.liquidate_position(
        &txid.to_string(),
        &payload.to_string(),
        lp_res.1,
        false,
    ) {
        Ok(res) => res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let _ = save_contract(&contract_1, payload, txid, pending);
    let _ = save_contract(&contract_2, payload, txid, pending);
    let mut balance_type = "O";
    if pending {
        balance_type = "U";
    }
    save_check_utxo_file(
        &contract_2.contractid,
        &res_2.0,
        res_2.1,
        res_2.2,
        pending,
        balance_type,
    );
    save_check_utxo_file(
        &contract_1.contractid,
        &res_1.0,
        res_1.1,
        res_1.2,
        pending,
        balance_type,
    );
    if !pending {
        let _ = save_contract(&contract_1, payload, txid, true);
        let _ = save_contract(&contract_2, payload, txid, true);
    }
}

pub async fn perform_liquidate_position_lp(
    txid: &str,
    payload: &str,
    pending: bool,
    lp_contract_id: &String,
    block_height: i32,
) {
    let mut lp_contract = match read_contract(lp_contract_id.as_str(), pending) {
        Ok(contract) => contract,
        Err(_) => return,
    };

    if lp_contract.payloads.contains_key(txid) {
        return;
    }

    let captures = match handle_liquidatation_payload_lp(&payload) {
        Ok(captures) => captures,
        Err(_) => return,
    };

    let input_utxos: Vec<String> = match get_tx_inputs(txid).await {
        Ok(inputs) => inputs,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let lp_res = match lp_contract.liquidate_postion_lp(
        &txid.to_string(),
        &payload.to_string(),
        &input_utxos,
        captures,
        block_height as u64,
    ) {
        Ok(lp_res) => lp_res,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let _ = save_contract(&lp_contract, payload, txid, pending);
    save_check_utxo_file(
        &lp_contract.contractid,
        &lp_res.2,
        lp_res.3,
        lp_res.4,
        pending,
        "O",
    );
    if !pending {
        let _ = save_contract(&lp_contract, payload, txid, true);
        for utxo in input_utxos {
            let file_path = format!("./Json/UTXOS/{}.txt", utxo);
            // Attempt to remove the file
            match fs::remove_file(file_path) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}

pub fn handle_provide_liquidity_payload_lp(payload: &str) -> Result<u64, String> {
    let words: Vec<&str> = payload.split("PLP[").collect();
    if words.len() < 2 {
        return Err("Invalid liquidity pool payload".to_string());
    }

    let amount_split = replace_payload_special_characters(&words[1].to_string());

    // Parse strings to numeric types
    let amount = match amount_split.parse() {
        Ok(amount) => amount,
        Err(_) => return Err("Not mint valid payload".to_string()),
    };
    return Ok(amount);
}

pub fn handle_swap_payload_lp(payload: &str) -> Result<(bool, u64, u64, f32), String> {
    let words: Vec<&str> = payload.split("SLP[").collect();
    if words.len() < 2 {
        return Err("Invalid liquidity pool payload".to_string());
    }

    let swap_split: Vec<&str> = words[1].split(",").collect();
    if swap_split.len() < 4 {
        return Err("Invalid liquidity pool payload".to_string());
    }

    let lp_contract: bool = swap_split[0].to_ascii_lowercase().contains("0");
    let amount_split = replace_payload_special_characters(&swap_split[1].to_string());
    let quoted_split = replace_payload_special_characters(&swap_split[2].to_string());
    let tolerance_split = replace_payload_special_characters(&swap_split[3].to_string());

    // Parse strings to numeric types
    let amount = match amount_split.parse() {
        Ok(amount) => amount,
        Err(_) => return Err("Not valid payload".to_string()),
    };

    let quoted = match quoted_split.parse() {
        Ok(quoted) => quoted,
        Err(_) => return Err("Not valid payload".to_string()),
    };

    let tolerance = match tolerance_split.parse() {
        Ok(tolerance) => tolerance,
        Err(_) => return Err("Not valid payload".to_string()),
    };

    return Ok((lp_contract, amount, quoted, tolerance));
}

pub fn handle_liquidatation_payload_lp(payload: &str) -> Result<u64, String> {
    let words: Vec<&str> = payload.split("LLP[").collect();
    if words.len() < 2 {
        return Err("Invalid liquidate position payload".to_string());
    }

    let amount_split = replace_payload_special_characters(&words[1].to_string());

    // Parse strings to numeric types
    let amount = match amount_split.parse() {
        Ok(amount) => amount,
        Err(_) => return Err("Not mint valid payload".to_string()),
    };
    return Ok(amount);
}

pub fn save_check_utxo_file(
    contract_id: &String,
    utxo: &String,
    amount: u64,
    drip_present: bool,
    pending: bool,
    balance_type: &str,
) {
    if amount == 0 && !drip_present {
        return;
    }

    let mut data = format!("{}:{}-,{}", &contract_id, balance_type, amount);
    if pending {
        data = format!("{}:P-{}-,{}", &contract_id, balance_type, amount);
        if drip_present {
            data = format!("{}:P-D{}-,{}", &contract_id, balance_type, amount);
        }
    } else if drip_present {
        data = format!("{}:D{}-,{}", &contract_id, balance_type, amount);
    }

    match fs::write(format!("./Json/UTXOS/{}.txt", &utxo), data.clone()) {
        Ok(_) => {}
        Err(_) => return,
    };
}
