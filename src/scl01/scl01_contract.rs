use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    pub max_supply: Option<u64>,
    pub liquidated_tokens: Option<u64>,
    pub liquidity_pool: Option<LiquidityPool>,
}


impl SCL01Contract {
    pub fn right_to_mint(&mut self, txid: &String, payload: &String, rtm: &String, reciever: &String, change_utxo: &String, mint_amount: &u64)-> Result<(String, u64, bool), String>{
        let mut right_to_mint = match self.right_to_mint.clone() {
            Some(right_to_mint) => right_to_mint,
            None => return Err("right_to_mint: no rights to mint for contract".to_string()),
        };

        let drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };
        
        let rights_amount = match right_to_mint.get(rtm){
            Some(rights_amount) => rights_amount.clone(),
            None => return Err("right_to_mint: rights not found".to_string()),
        };

        let change: u64 = rights_amount - mint_amount;
        let mut amount_to_mint = rights_amount;
        if change  > 0 {
            amount_to_mint =  *mint_amount;
            right_to_mint.insert(change_utxo.to_string(), change);
        }

        let mut new_owner: (String, u64, bool) = (reciever.to_string(), 0, false);
        match drips.get(reciever){
            Some(_) => new_owner.2 = true,
            None => new_owner.2 = false,
        };
        match self.owners.get(reciever) {
            Some(&e) => {
                self.owners.insert(reciever.clone(), &e + amount_to_mint);
                new_owner.1 = &e + amount_to_mint;
            }
            None => {
                self.owners.insert(reciever.clone(), amount_to_mint);
                new_owner.1 = amount_to_mint;
            }
        }

        self.supply += amount_to_mint;
        right_to_mint.remove(rtm);
        self.right_to_mint = Some(right_to_mint);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(new_owner)
    }

    pub fn consolidate(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, receivers: &Vec<String>, current_block_height: u64) -> Result<(bool, u64), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }
        if owners_amount == 0 {
            return Err("consolidate: owner amount is zero".to_string());
        }
        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

         let mut new_drips: Vec<Drip> = Vec::new();
            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo.clone()) {
                    self.owners.remove(&sender_utxo);
                    if let Some(old_drips) = drips.get(&sender_utxo) {
                        for drip in old_drips {
                            let new_drip = Drip {
                                block_end: drip.block_end.clone(),
                                drip_amount: drip.drip_amount.clone(),
                                amount: drip.amount.clone() - (current_block_height - drip.start_block) * drip.drip_amount,
                                start_block: current_block_height,
                                last_block_dripped: current_block_height
                            };

                            new_drips.push(new_drip.clone());
                        }
                        
                        // Remove the old drip from the vector
                        drips.remove(&sender_utxo);
                    }
                }
            }

            let last_index = receivers.len() - 1;
            drips.insert(receivers[last_index].clone(),new_drips);

            let mut recievers_drips_present = false; 
            for entry in receivers.clone() {
                match self.owners.get(&entry) {
                    Some(&e) => self.owners.insert(entry.clone(), &e + owners_amount),
                    None => self.owners.insert(entry.clone(), owners_amount)
                };

                if drips.contains_key(&entry) {
                    let blocks_dripped = owners_amount;
                    match self.owners.get(&entry) {
                        Some(&e) => self.owners.insert(entry.clone(), &e + blocks_dripped),
                        None => self.owners.insert(entry.clone(), owners_amount),
                    };

                    recievers_drips_present = true;
                }else{
                    recievers_drips_present = false;
                }
            }

            self.payloads.insert(txid.to_string(), payload.to_string());
            self.drips = Some(drips);
            return Ok((recievers_drips_present, owners_amount));
    }

    pub fn transfer(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, receivers: &Vec<(String, u64)>, current_block_height: u64) -> Result<(Vec<bool>, u64), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }
        if owners_amount == 0 {
            return Err("transfer: owner amount is zero".to_string());
        }
        let mut total_value: u64 = 0;
        for entry in receivers.clone() {
            total_value += entry.1;
        }

        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        if total_value <= owners_amount {
            let mut new_drips: Vec<Drip> = Vec::new();
            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo.clone()) {
                    self.owners.remove(&sender_utxo);
                    if let Some(old_drips) = drips.get(&sender_utxo) {
                        for drip in old_drips {
                            let new_drip = Drip {
                                block_end: drip.block_end.clone(),
                                drip_amount: drip.drip_amount.clone(),
                                amount: drip.amount.clone() - (current_block_height - drip.start_block) * drip.drip_amount,
                                start_block: current_block_height,
                                last_block_dripped: current_block_height
                            };

                            new_drips.push(new_drip.clone());
                        }
                        
                        // Remove the old drip from the vector
                        drips.remove(&sender_utxo);
                    }
                }
            }

            let last_index = receivers.len() - 1;
            let mut drip_ret = 0;
            if !new_drips.is_empty() {
                let last_receiver = &receivers[last_index].0.clone();       
                drips.insert(last_receiver.clone(), new_drips);
                let blocks_dripped = owners_amount - total_value;
            
                if let Some(owned) = self.owners.get_mut(last_receiver) {
                    *owned += blocks_dripped;
                    drip_ret = *owned;
                }
            }
            
            

            let mut recievers_drips_present: Vec<bool> = Vec::new(); 
            for entry in receivers.clone() {
                match self.owners.get(&entry.0) {
                    Some(&e) => self.owners.insert(entry.0.clone(), &e + entry.1),
                    None => self.owners.insert(entry.0.clone(), entry.1)
                };

                if drips.contains_key(&entry.0) {
                    recievers_drips_present.push(true);
                }else{
                    recievers_drips_present.push(false);
                }
            }

            self.payloads.insert(txid.to_string(), payload.to_string());
            self.drips = Some(drips);
            return Ok((recievers_drips_present, drip_ret));
        } else{
            return Err("transfer: owner amount is less than recievers total".to_string());
        }
    }

    pub fn start_drip(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, receivers: &HashMap<String, (u64,u64)>, change_utxo: &String, current_block_height:u64) -> Result<(Vec<(String, u64)>, (String, u64)), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }

        if owners_amount == 0 {
            return Err("start_drip: owner amount is zero".to_string());
        }

        let mut total_value: u64 = 0;
        for entry in receivers.clone() {
            total_value += entry.1.0;
        }

        if total_value <= owners_amount {
            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo.clone()) {
                    self.owners.remove(&sender_utxo);
                }
            }

            let mut d = match self.drips.clone() {
                Some(d) => d,
                None => HashMap::new(),
            };

            let change = owners_amount - total_value;
            let mut new_owner = (change_utxo.to_string(), 0);
            if change > 0 {
                if self.owners.contains_key(change_utxo) {
                    let new_amount = self.owners[change_utxo] + change;
                    new_owner.1 = new_amount.clone();
                    self.owners.insert(change_utxo.to_string(), new_amount);
                } else {
                    new_owner.1 = change;
                    self.owners.insert(change_utxo.to_string(), change);
                }
            }


            let mut drippers: Vec<(String, u64)> =  Vec::new();
            let mut block_drip = 0;
            for entry in receivers.clone() {
                let drip_amt = (entry.1.0)/(entry.1.1);
                let drip = Drip{
                     block_end: current_block_height + entry.1.1 - 1,
                     drip_amount: drip_amt.clone(),
                     amount: entry.1.0.clone(),
                     start_block: current_block_height.clone(),
                     last_block_dripped: current_block_height
                };

                let mut drips = Vec::new();
                drips.push(drip);
                d.insert(entry.0.clone(), drips);
                let mut drip_balance:(String, u64) = (entry.0.clone(), drip_amt);
                match self.owners.get(&entry.0) {
                    Some(&existing_amount) => {
                        self.owners.insert(entry.0, &existing_amount + drip_amt);
                        drip_balance.1 = existing_amount+ drip_amt;
                    }
                    None => {
                        self.owners.insert(entry.0, drip_amt);
                    }
                }

                block_drip += drip_amt;
                drippers.push(drip_balance);
        }
        
        self.drips = Some(d);
        self.supply -= total_value - block_drip;
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok((drippers, new_owner));
        }

        return Err("start_drip: owner amount is zero".to_string());
    }
    
    pub fn drip(&mut self, current_block_height: u64)-> Result<Vec<(String, u64, bool)>,String>{
        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => return Ok(Vec::new()),
        };
        
        let mut new_owners: Vec<(String, u64, bool)> = Vec::new();
        for (utxo, drips_on_utxo) in drips.clone() {
            let mut updated_drips: Vec<Drip> = Vec::new();
            let mut new_owner = (utxo.to_string(),0, true);
            for mut drip in drips_on_utxo{
                let mut current_block = current_block_height;
                if current_block_height > drip.block_end {
                    current_block = drip.block_end
                }

                let mut drip_amount = (current_block - drip.last_block_dripped) * drip.drip_amount;
                if current_block == drip.block_end && ((drip.block_end  - drip.start_block) + 1) * drip.drip_amount < drip.amount  {
                    drip_amount += drip.amount - (drip.block_end  - drip.start_block + 1) * drip.drip_amount;
                }

                match self.owners.get(&utxo) {
                    Some(&e) => {
                        self.owners.insert(utxo.clone(), &e + drip_amount);
                        new_owner.1 = &e + drip_amount;
                    }
                    None => {
                        self.owners.insert(utxo.clone(), drip_amount);
                        new_owner.1 = drip_amount;
                    }
                }

                self.supply += drip_amount;
                drip.last_block_dripped = current_block;
                if current_block < drip.block_end {
                    updated_drips.push(drip);
                }else{
                    new_owner.2 = false;
                }
            }


            if new_owner.1 != 0 {
                new_owners.push(new_owner);
            }
        
            if updated_drips.len() >= 1 {
                drips.insert(utxo, updated_drips);
            }else{
                drips.remove(&utxo);
            }
        }
        self.drips = Some(drips);
        return Ok(new_owners)
    }
    
    pub fn burn(&mut self, txid: &String, payload: &String, burner_utxos: &Vec<String>, burn_amount: &u64, change_utxo: &String) -> Result<i32, String> {
        let mut owners_amount = 0;
        for burner_utxo in burner_utxos.iter() {
            if let Some(&amount) = self.owners.get(burner_utxo) {
                owners_amount += amount;
            }
        }
        
        if owners_amount == 0 {
            return Err("burn: owner has no tokens to burn".to_string());
        }

        if owners_amount >= *burn_amount {
            for burner_utxo in burner_utxos {
                if let Some(&_amount) = self.owners.get(burner_utxo) {
                    self.owners.remove(burner_utxo);
                }
            }

            self.owners.insert(change_utxo.to_string(), owners_amount - *burn_amount);
            self.supply -= *burn_amount;
            self.payloads.insert(txid.to_string(), payload.to_string());
        } else {
            return Err("burn: trying to brun more than is owned".to_string());
        }
        Ok(0)
    }

    pub fn list(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, new_listing: Listing, current_block_height:u64) -> Result<(String,u64,bool), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }
        if owners_amount == 0 {
            return Err("list: owner amount is zero".to_string());
        }

        if sender_utxos.len() == 0 {
            return Err("list: no senders".to_string());
        }

        let mut new_owner = (new_listing.change_utxo.to_string(),0,false);

        if new_listing.list_amt <= owners_amount {
            let mut drips = match self.drips.clone() {
                Some(drips) => drips,
                None => HashMap::new(),
            };

            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo) {
                    self.owners.remove(&sender_utxo);
                }

                if let Some(old_drips) = drips.get(&sender_utxo) {
                    let mut new_drips: Vec<Drip> = Vec::new();
                    for drip in old_drips {
                        let new_drip = Drip {
                            block_end: drip.block_end.clone(),
                            drip_amount: drip.drip_amount.clone(),
                            amount: drip.amount.clone() - (current_block_height - drip.start_block) * drip.drip_amount,
                            start_block: current_block_height,
                            last_block_dripped:current_block_height.clone()
                        };
                        new_drips.push(new_drip.clone());
                    }

                    drips.insert(new_listing.change_utxo.clone(),new_drips);
                    drips.remove(&sender_utxo);
                    new_owner.2 = true;
                }
            }

            let change_amt: u64 = owners_amount - new_listing.list_amt;
            if change_amt > 0 {
                if self.owners.contains_key(&new_listing.change_utxo) {
                    let new_amount = self.owners[&new_listing.change_utxo] + change_amt;
                    new_owner.1 = new_amount.clone();
                    self.owners.insert(new_listing.change_utxo.to_string(), new_amount);
                } else {
                    new_owner.1 = change_amt.clone();
                    self.owners.insert(new_listing.change_utxo.to_string(), change_amt);
                }
            }
            
            let order_id: String = sender_utxos[0].to_string();
            let mut listing = match self.listings.clone() {
                Some(listings) => listings,
                None => HashMap::new(),
            };

            listing.insert(order_id, new_listing.clone());
            self.listings = Some(listing);
            self.drips = Some(drips);
            self.payloads.insert(txid.to_string(), payload.to_string());
        }
        return Ok(new_owner);
    }

    pub fn bid(&mut self, txid: &String, payload: &String, bids: Vec<Bid>, bidding_ids: &Vec<String>, current_block_height: i32) -> Result<i32, String> {
        let mut listings_available = match self.listings.clone() {
            Some(listings_available) => listings_available,
            None => return Err("bid: no listings for contract".to_string()),
        };

        let mut bids_available = match self.bids.clone() {
            Some(bids_available) => bids_available,
            None => HashMap::new(),
        };

        for (i, _) in bids.iter().enumerate() {
            if listings_available.clone().contains_key(&bids[i].order_id) {
                if bids[i].bid_amount > listings_available[&bids[i].order_id].list_amt {
                    continue;
                }

                if bids[i].bid_amount/ 10u64.pow(self.decimals as u32)  * bids[i].bid_price  >=  listings_available[&bids[i].order_id].list_amt / 10u64.pow(self.decimals as u32)  * listings_available[&bids[i].order_id].price{
                    let mut listing = listings_available[&bids[i].order_id].clone();
                    listing.valid_bid_block = Some(current_block_height);
                    listings_available.insert(bids[i].order_id.to_string(), listing);
                    self.listings = Some(listings_available.clone());
                }
                
                bids_available.insert(bidding_ids[i].to_string(), bids[i].clone());
                self.bids = Some(bids_available.clone());
            }
        }

        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(0);
    }

    pub fn accept_bid(&mut self, txid: &String, payload: &String, bid_id: &String) -> Result<i32, String> {
        let bids_available = match self.bids.clone() {
            Some(bids_available) => bids_available,
            None => return Err("accept_bid: no bids for contract".to_string()),
        };

        let mut fulfillments = match self.fulfillments.clone() {
            Some(fulfillments) => fulfillments,
            None => HashMap::new(),
        };

        if bids_available.contains_key(bid_id) {
            let order_id: String = bids_available[bid_id].order_id.clone();
            fulfillments.insert(bid_id.to_string(), order_id);
            self.fulfillments = Some(fulfillments.clone());
            let payload_data = format!("{}-ExtraInfo-{},{},{}",payload, bid_id, bids_available[bid_id].bid_amount, bids_available[bid_id].bid_price);
            self.payloads.insert(txid.to_string(), payload_data);
        }
        
        return Ok(0);
    }

    pub fn fulfil(&mut self, txid: &String, payload: &String, bid_id: &String) -> Result<(HashMap<String, u64>, Vec<String>, String), String> {
        let mut bids_available = match self.bids.clone() {
            Some(bids_available) => bids_available,
            None => return Err("accept_bid: no bids for contract".to_string()),
        };

        let mut fulfillments = match self.fulfillments.clone() {
            Some(fulfillments) => fulfillments,
            None => HashMap::new(),
        };

        let mut listing = match self.listings.clone() {
            Some(listings) => listings,
            None => HashMap::new(),
        };

        let mut new_owners = HashMap::<String,u64>::new();
        let mut listing_removed = String::new();
        let mut bids_removed = Vec::<String>::new();
        if fulfillments.clone().contains_key(bid_id) {
            let order_id = fulfillments[bid_id].clone();
            let bid = bids_available[bid_id].clone();
            let recievers_utxo = format!("{}:0",txid);
            if self.owners.contains_key(&recievers_utxo) {
                let new_amount = self.owners[&recievers_utxo] + bid.bid_amount;
                self.owners.insert(recievers_utxo.to_string(), new_amount);
                new_owners.insert(recievers_utxo.to_string(), new_amount);
            } else {
                self.owners.insert(recievers_utxo.to_string(), bid.bid_amount);
                new_owners.insert(recievers_utxo.to_string(), bid.bid_amount);
            }

            if listing[&order_id].list_amt > bid.bid_amount {
                let change = format!("{}:2",txid);
                let change_amount = listing[&order_id].list_amt - bid.bid_amount;
                if self.owners.contains_key(&change) {
                    let new_amount = self.owners[&change] + change_amount;
                    self.owners.insert(change.to_string(), new_amount);
                    new_owners.insert(change.to_string(), new_amount);
                } else {
                    self.owners.insert(change.to_string(), change_amount);
                    new_owners.insert(change.to_string(), change_amount);
                }
            }
         
            listing_removed = listing[&order_id].list_utxo.clone();
            fulfillments.remove(bid_id);
            listing.remove(&order_id);
            let payload_data = format!("{}-ExtraInfo-{},{},{}",payload, bid_id, bids_available[bid_id].bid_amount, bids_available[bid_id].bid_price);

            for (key, value) in bids_available.clone().iter_mut() {
                if value.order_id == order_id.to_string() {
                    bids_available.remove(key);
                    bids_removed.push(value.reseved_utxo.clone());
                }
            }

            self.bids = Some(bids_available.clone());
            self.fulfillments = Some(fulfillments.clone());
            self.listings = Some(listing.clone());
            self.payloads.insert(txid.to_string(), payload_data);
        }

        return Ok((new_owners, bids_removed, listing_removed));
    }

    pub fn cancel_listing(&mut self, txid: &String, listing_utxo: &String,  payload: String) -> Result<((String, u64), Vec<String>), String> {
        let mut bids_available = match self.bids.clone() {
            Some(bids_available) => bids_available,
            None => HashMap::new(),
        };

        let fulfillments = match self.fulfillments.clone() {
            Some(fulfillments) => fulfillments,
            None => HashMap::new(),
        };

        let mut listings = match self.listings.clone() {
            Some(listings) => listings,
            None => return Err("cancel_listing: no listings for contract".to_string()),
        };

        let mut canceled_listing = Listing::default();
        let mut order_id = String::new();
        for (key, value) in listings.clone() {
            if value.list_utxo == listing_utxo.to_string() {
                canceled_listing = value.clone();
                order_id = key.clone();
                break;
            }
        }

        for (_, value) in fulfillments {
            if value == order_id{
                return Err("cancel_listing: order has been fulfilled".to_string());
            }
        }

        let mut bids_removed = Vec::<String>::new();
        let recievers_utxo: String = format!("{}:0",txid);
        let mut new_owner = (recievers_utxo.to_string(), canceled_listing.list_amt);
        if self.owners.contains_key(&recievers_utxo) {
            let new_amount = self.owners[&recievers_utxo] + canceled_listing.list_amt;
            self.owners.insert(recievers_utxo.to_string(), new_amount);
            new_owner.1 = new_amount;
        } else {
            self.owners.insert(recievers_utxo.to_string(), canceled_listing.list_amt);
        }

        listings.remove(&order_id);

        for (key, value) in bids_available.clone().iter_mut() {
            if value.order_id == order_id.to_string() {
                bids_available.remove(key);
                bids_removed.push(value.reseved_utxo.clone());
            }
        }

        self.bids = Some(bids_available.clone());
        self.listings = Some(listings.clone());
        self.payloads.insert(txid.to_string(), payload);
        return Ok((new_owner, bids_removed));
    }

    pub fn cancel_bid(&mut self, txid: &String, bidding_utxo: &String, payload: String) -> Result<i32, String> {
        let mut bids_available = match self.bids.clone() {
            Some(bids_available) => bids_available,
            None => return Err("cancel_bid: no bids for contract".to_string()),
        };

        let fulfillments = match self.fulfillments.clone() {
            Some(fulfillments) => fulfillments,
            None => HashMap::new(),
        };

        match self.listings.clone() {
            Some(listings) => listings,
            None => return Err("cancel_bid: no listings for contract".to_string()),
        };

        let mut bid_id = String::new();
        for (key, value) in bids_available.clone() {
            if value.reseved_utxo == bidding_utxo.to_string() {
                bid_id = key.clone();
                break;
            }
        }

        if fulfillments.contains_key(&bid_id) {
            return Err("cancel_bid: order has been fulfilled".to_string());
        }

        bids_available.remove(&bid_id);

        self.bids = Some(bids_available.clone());
        self.payloads.insert(txid.to_string(), payload);
        return Ok(0);
    }
    
    pub fn airdop(&mut self, txid: &String, payload: &String, receiver: &String, pending: bool) -> Result<u64, String> {
        let current_airdrops = match self.current_airdrops.clone() {
            Some(current_airdrops) => current_airdrops,
            None =>  return  Err("airdop: no airdrops".to_string()),
        };

        let airdrop_amount = match self.airdrop_amount.clone() {
            Some(airdrop_amount) => airdrop_amount,
            None =>  return  Err("airdop: no airdrops".to_string()),
        };

        let total_airdrops = match self.total_airdrops.clone() {
            Some(total_airdrops) => total_airdrops,
            None =>  return  Err("airdop: no Airdrops".to_string()),
        };

        if current_airdrops >= total_airdrops {
            return Err("airdop: contract has reached max supply".to_string());
        }

        let mut owner_amount = airdrop_amount;

        if current_airdrops + 1 == total_airdrops {
            let mut last_airdrop_split = match self.last_airdrop_split.clone() {
                Some(last_airdrop_split) => last_airdrop_split,
                None =>  Vec::new()
            };

            last_airdrop_split.push(receiver.to_string());
            self.last_airdrop_split = Some(last_airdrop_split);
            self.payloads.insert(txid.to_string(), payload.to_string());
            return Ok(owner_amount);
        }

        let mut p_c = match self.pending_claims.clone() {
                Some(p_c) => p_c,
                None => HashMap::new(),
        };

        if pending {
           p_c.insert(receiver.to_string(), airdrop_amount);
           if let Some(owned) = self.owners.get(receiver){
                owner_amount += owned;    
           }
        } else {
            p_c.remove(receiver);
            match self.owners.get(receiver) {
                Some(&e) => {
                    self.owners.insert(receiver.to_string(), &e + airdrop_amount);
                    owner_amount += e;               
                }
                None => {
                    self.owners.insert(receiver.to_string(), airdrop_amount);
                }
            }
        }
      
        self.current_airdrops = Some(current_airdrops + 1);
        self.pending_claims = Some(p_c);
        self.payloads.insert(txid.to_string(), payload.to_string());
        self.supply += airdrop_amount;
        return Ok(owner_amount);
    }

    pub fn airdop_split(&mut self) -> Result<Vec<(String, u64)>, String> {
        let max_supply = match self.max_supply.clone() {
            Some(max_supply) => max_supply,
            None =>  return  Err("airdop_split: no max supply".to_string()),
        };

        let airdrop_amount = match self.airdrop_amount.clone() {
            Some(airdrop_amount) => airdrop_amount,
            None =>  return  Err("airdop_split: no airdrops".to_string()),
        };

        let last_airdrop_split = match self.last_airdrop_split.clone() {
            Some(last_airdrop_split) => last_airdrop_split,
            None =>  return  Ok(Vec::new())
        };

        let split_amount = airdrop_amount/ last_airdrop_split.len() as u64;
        let mut new_owners:Vec<(String, u64)> = Vec::new(); 
        for receiver in last_airdrop_split {
            let new_owner: (String, u64) = (receiver.to_string(), split_amount);
            self.owners.insert(receiver.to_string(), split_amount);
            new_owners.push(new_owner);
        }

        self.current_airdrops = self.total_airdrops;
        self.last_airdrop_split = None;
        self.supply = max_supply;
        return Ok(new_owners);
    }
    
    pub fn create_dim_airdrop(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, pool_amount: &u64, step_down_amount: &u64, step_period_amount: &u64, max_airdrop: &u64, min_airdrop: &u64, change_utxo: &String, single_drop: &bool, current_block_height: u64) -> Result<(String, u64, bool), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }

        if owners_amount == 0 {
            return Err("create_dim_airdrop: owner amount is zero".to_string());
        }

        if pool_amount > &owners_amount {
            return Err("create_dim_airdrop: pool amount is more than the owned amount".to_string());
        }

        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        let mut new_owner = (change_utxo.to_string(),0, false);
        let mut diminishing_airdrops = match self.diminishing_airdrops.clone() {
            Some(diminishing_airdrops) => diminishing_airdrops,
            None => HashMap::new(),
        };

        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo.clone()) {
                self.owners.remove(&sender_utxo);
                if let Some(old_drips) = drips.get(&sender_utxo) {
                    let mut new_drips: Vec<Drip> = Vec::new();
                    for drip in old_drips {
                        let new_drip = Drip {
                            block_end: drip.block_end.clone(),
                            drip_amount: drip.drip_amount.clone(),
                            amount: drip.amount.clone() - (current_block_height - drip.start_block) * drip.drip_amount,
                            start_block: current_block_height,
                            last_block_dripped:current_block_height.clone()
                        };
                        new_drips.push(new_drip.clone());
                    }

                    drips.insert(change_utxo.clone(),new_drips);
                    // Remove the old drip from the vector
                    drips.remove(&sender_utxo);
                    new_owner.2 = true;
                }
            }
        }

        let change_amt: u64 = owners_amount - pool_amount;
        if change_amt > 0 {
            if self.owners.contains_key(change_utxo) {
                let new_amount = self.owners[change_utxo] + change_amt;
                new_owner.1 = new_amount.clone();
                self.owners.insert(change_utxo.to_string(), new_amount);
            } else {
                new_owner.1 = change_amt.clone();
                self.owners.insert(change_utxo.to_string(), change_amt);
            }
        }

        let dim_airdrop = DimAirdrop {
            pool_amount: *pool_amount,
            step_down_amount: *step_down_amount,
            step_period_amount: *step_period_amount,
            max_airdrop: *max_airdrop,
            min_airdrop: *min_airdrop,
            current_airdrop: *max_airdrop,
            current_in_period: 0,
            amount_airdropped: 0,
            last_airdrop_split: None,
            claimers: HashMap::new(),
            single_drop: *single_drop,
        };

        diminishing_airdrops.insert(sender_utxos[0].clone(), dim_airdrop);

        self.payloads.insert(txid.to_string(), payload.to_string());
        self.diminishing_airdrops = Some(diminishing_airdrops);
        self.supply -= pool_amount;
        self.drips = Some(drips);
        return Ok(new_owner);
    }

    pub fn claim_dim_airdrop(&mut self, txid: &String, payload: &String, claim_id: &String, reciever_utxo: &String, pending: bool, donater_pub_address: &String) -> Result<(String, u64, bool), String> {
        let mut diminishing_airdrops = match self.diminishing_airdrops.clone() {
            Some(diminishing_airdrops) => diminishing_airdrops,
            None => return Err("claim_dim_airdrop: contract has reached no claimable diminsihing airdrops".to_string()),
        };

        let mut dim_airdrop: DimAirdrop =  match diminishing_airdrops.get(claim_id) {
            Some(dim_airdrop) => dim_airdrop.clone(),
            None => return Err("claim_dim_airdrop: diminishing airdrop claim id not found".to_string()),
        };

        let mut new_owner = (reciever_utxo.to_string(), 0, false);
        if dim_airdrop.step_period_amount == dim_airdrop.current_in_period {
            dim_airdrop.current_in_period = 0;
            if dim_airdrop.current_airdrop > dim_airdrop.min_airdrop {
                dim_airdrop.current_airdrop -= dim_airdrop.step_down_amount; 
            }
        }

        let mut airdrop_amount = dim_airdrop.current_airdrop;
        if dim_airdrop.amount_airdropped + dim_airdrop.current_airdrop >= dim_airdrop.pool_amount {
            airdrop_amount = dim_airdrop.pool_amount  - dim_airdrop.amount_airdropped;
        }

        let drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        let mut pending_claims = match self.pending_claims.clone() {
            Some(pending_claims) => pending_claims,
            None => HashMap::new(),
        };

        if pending {
            pending_claims.insert(reciever_utxo.to_string(), airdrop_amount);
            let mut new_amount = airdrop_amount;
            if self.owners.contains_key(reciever_utxo) {
                new_amount = self.owners[reciever_utxo] + airdrop_amount;
            }

            new_owner.1 = new_amount;
         } else {
            pending_claims.remove(reciever_utxo);
             if self.owners.contains_key(reciever_utxo) {
                let new_amount = self.owners[reciever_utxo] + airdrop_amount;
                new_owner.1 = new_amount.clone();
                self.owners.insert(reciever_utxo.to_string(), new_amount);
            } else {
                new_owner.1 = airdrop_amount;
                self.owners.insert(reciever_utxo.to_string(), airdrop_amount);
            }
         }

        if drips.contains_key(reciever_utxo) {
            new_owner.2 = true;
        }

        if dim_airdrop.single_drop {
            dim_airdrop.claimers.insert(donater_pub_address.to_string(), airdrop_amount);
        }

        dim_airdrop.amount_airdropped += airdrop_amount;
        dim_airdrop.current_in_period += 1;
        if dim_airdrop.amount_airdropped == dim_airdrop.pool_amount  {
            diminishing_airdrops.remove(claim_id);      
        }else{
            diminishing_airdrops.insert(claim_id.to_string(), dim_airdrop);
        }
        
        self.supply += airdrop_amount;
        self.diminishing_airdrops = Some(diminishing_airdrops);
        self.pending_claims = Some(pending_claims);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(new_owner);
    }
    
    pub fn create_dge(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, dge: DGE, change_utxo: &String, current_block_height: u64) -> Result<(String, u64, bool), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }
        if owners_amount == 0 {
            return Err("create_dge: owner amount is zero".to_string());
        }

        if dge.pool_amount > owners_amount {
            return Err("create_dge: pool amount is more than the owned amount".to_string());
        }

        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        let mut new_owner = (change_utxo.to_string(),0, false);
        let mut dges = match self.dges.clone() {
            Some(dges) => dges,
            None => HashMap::new(),
        };

        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo.clone()) {
                self.owners.remove(&sender_utxo);
                if let Some(old_drips) = drips.get(&sender_utxo) {
                    let mut new_drips: Vec<Drip> = Vec::new();
                    for drip in old_drips {
                        let new_drip = Drip {
                            block_end: drip.block_end.clone(),
                            drip_amount: drip.drip_amount.clone(),
                            amount: drip.amount.clone() - (current_block_height - drip.start_block) * drip.drip_amount,
                            start_block: current_block_height,
                            last_block_dripped:current_block_height.clone()
                        };
                        new_drips.push(new_drip.clone());
                    }

                    drips.insert(change_utxo.clone(),new_drips);
                    // Remove the old drip from the vector
                    drips.remove(&sender_utxo);
                    new_owner.2 = true;
                }
            }
        }

        let change_amt: u64 = owners_amount - dge.pool_amount;
        if change_amt > 0 {
            if self.owners.contains_key(change_utxo) {
                let new_amount = self.owners[change_utxo] + change_amt;
                new_owner.1 = new_amount.clone();
                self.owners.insert(change_utxo.to_string(), new_amount);
            } else {
                new_owner.1 = change_amt.clone();
                self.owners.insert(change_utxo.to_string(), change_amt);
            }
        }

        dges.insert(sender_utxos[0].clone(), dge.clone());

        self.payloads.insert(txid.to_string(), payload.to_string());
        self.supply -= dge.pool_amount;
        self.dges = Some(dges);
        return Ok(new_owner);
    }

    pub fn claim_dge(&mut self, txid: &String, payload: &String, claim_id: &String, reciever_utxo: &String, donater: &String, donation: u64, current_block_height: u64) -> Result<(String, u64), String> {
        let mut dges = match self.dges.clone() {
            Some(dges) => dges,
            None => return Err("claim_dge: contract has reached no claimable dges".to_string()),
        };

        let mut dge: DGE =  match dges.get(claim_id) {
            Some(dge) => dge.clone(),
            None => return Err("claim_dge: dge claim id not found".to_string()),
        };

        if donation as u128 > (dge.max_drop as u128 * dge.sats_rate as u128) / 10u64.pow(self.decimals as u32) as u128 {
            return Err("claim_dge: donation over maximum limit".to_string())
        }

        let mut new_owner = (reciever_utxo.to_string(), 0);
        let mut token_amount = donation* 10u64.pow(self.decimals as u32)/ (dge.sats_rate);
        if token_amount == 0 {
            return Err("claim_dge: token allocation is zero".to_string())
        }

        if token_amount + dge.current_amount_dropped >= dge.pool_amount {
           token_amount = dge.pool_amount - dge.current_amount_dropped;  
        }

        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        let drip_amount = token_amount / dge.drip_duration;
        let drip = Drip{
             block_end: current_block_height + dge.drip_duration -1,
             drip_amount: drip_amount.clone(),
             amount: token_amount.clone(),
             start_block: current_block_height.clone(),
             last_block_dripped:current_block_height.clone()
        };
        
        let mut new_drips = Vec::new();
        new_drips.push(drip);
        drips.insert(reciever_utxo.clone(), new_drips);
        match self.owners.get(reciever_utxo) {
            Some(&existing_amount) => {
                self.owners.insert(reciever_utxo.to_string(), &existing_amount + drip_amount);
                new_owner.1 = &existing_amount + drip_amount;
            }
            None => {
                self.owners.insert(reciever_utxo.to_string(), drip_amount);
                new_owner.1 = drip_amount;
            }
        }
        
        self.supply += drip_amount;
        self.drips = Some(drips);
        dge.current_amount_dropped += token_amount;
        if dge.single_drop {
            dge.donaters.insert(donater.to_string(), donation);
        }
        
        dges.insert(claim_id.to_string(), dge);
        self.dges = Some(dges);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(new_owner);
    }

    pub fn provide_liquidity(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, liquidation_amount: u64, block_height: u64, is_contract_1: bool) -> Result<(String,u64,bool), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }
        if owners_amount == 0 {
            return Err("provide_liquidity: owner amount is zero".to_string());
        }

        if liquidation_amount <= owners_amount {
            let mut owned_senders: Vec<String> = Vec::new();
            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo.clone()) {
                    self.owners.remove(&sender_utxo);
                    owned_senders.push(sender_utxo);
                }
            }

            let mut change_utxo = format!("{}:1",txid);
            if !is_contract_1 {
                change_utxo = format!("{}:2",txid);
            }

            let change_drip_present = self.transfer_drips(&owned_senders, block_height, &change_utxo);
            let liquidated_tokens = match self.liquidated_tokens.clone() {
                Some(liquidated_tokens) => liquidated_tokens,
                None => 0,
            };

            let change = owners_amount - liquidation_amount;
            if change > 0 {
                match self.owners.get(&change_utxo) {
                    Some(&e) => self.owners.insert(change_utxo.clone(), &e + change),
                    None => self.owners.insert(change_utxo.clone(), change)
                };
            }

            self.payloads.insert(txid.to_string(), payload.to_string());
            self.liquidated_tokens = Some(liquidated_tokens + liquidation_amount);
            return Ok((change_utxo, change, change_drip_present));
        } else{
            return Err("provide_liquidity: owner amount is less than the liquidation amount ".to_string());
        }
    }

    pub fn swap_claim(&mut self, txid: &String, payload: &String, sender_utxos: &Vec<String>, swap_amount: u64, block_height: u64) -> Result<(String, u64, bool), String> {
        let mut owners_amount: u64 = 0;
        for sender_utxo in sender_utxos.clone() {
            if self.owners.contains_key(&sender_utxo) {
                owners_amount += self.owners[&sender_utxo];
            }
        }

        let liquidated_tokens = match self.liquidated_tokens.clone() {
            Some(liquidated_tokens) => liquidated_tokens,
            None => 0,
        };

        if owners_amount == 0 {
            return Err("swap_claim: owner amount is zero".to_string());
        }

        if swap_amount <= owners_amount {
            for sender_utxo in sender_utxos.clone() {
                if self.owners.contains_key(&sender_utxo.clone()) {
                    self.owners.remove(&sender_utxo);
                }
            }

            let change_utxo = format!("{}:1", txid);
            let change_drip_present = self.transfer_drips(sender_utxos, block_height, &change_utxo);
            let change = owners_amount - swap_amount;
            if change > 0 {
                match self.owners.get(&change_utxo) {
                    Some(&e) => self.owners.insert(change_utxo.clone(), &e + change),
                    None => self.owners.insert(change_utxo.clone(), change)
                };
            }

            self.payloads.insert(txid.to_string(), payload.to_string());
            self.liquidated_tokens = Some(liquidated_tokens + swap_amount);
            return Ok((change_utxo, change, change_drip_present));
        } else{
            return Err("swap_claim: owner amount is less swapped amount ".to_string());
        }
    }

    pub fn swap_recieve(&mut self, txid: &String, payload: &String, swap_amount: u64) -> Result<(String, u64), String> {
        if swap_amount == 0 {
            return Err("swap_recieve: swap was not within tolerance".to_string());
        }

        let liquidated_tokens = match self.liquidated_tokens.clone() {
            Some(liquidated_tokens) => liquidated_tokens,
            None => return Err("swap_recieve: contract has no lp tokens assigned".to_string()),
        };

        let reciever_utxo = format!("{}:0", txid);
        match self.owners.get(&reciever_utxo) {
            Some(&e) => self.owners.insert(reciever_utxo.clone(), &e + swap_amount),
            None => self.owners.insert(reciever_utxo.clone(), swap_amount)
        };

        self.liquidated_tokens = Some(liquidated_tokens - swap_amount);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok((reciever_utxo, swap_amount));
    }

    pub fn liquidate_position(&mut self, txid: &String, payload: &String, amount: u64, is_contract_1:bool) -> Result<(String, u64, bool), String> {
        let mut liquidated_tokens = match self.liquidated_tokens.clone() {
            Some(liquidated_tokens) => liquidated_tokens,
            None => return Err("liquidate_position: contract has no lp tokens assigned".to_string()),
        };

        let drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        liquidated_tokens -= amount;
        let mut reciever_utxo: String =  format!("{}:1", txid);
        if !is_contract_1 {
            reciever_utxo =  format!("{}:2", txid);
        }
        let mut new_owner = (reciever_utxo.to_string(), 0, false);
        if self.owners.contains_key(&reciever_utxo) {
            let new_amount = self.owners[&reciever_utxo] + amount;
            new_owner.1 = new_amount.clone();
            self.owners.insert(reciever_utxo.to_string(), new_amount);
        } else {
            new_owner.1 = amount;
            self.owners.insert(reciever_utxo.to_string(), amount);
        }

        if drips.contains_key(&reciever_utxo) {
            new_owner.2 = true;
        }

        self.liquidated_tokens = Some(liquidated_tokens);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(new_owner);
    }

    pub fn provide_liquidity_lp(&mut self, txid: &str, payload: &str, provided_amount: u64)-> Result<(String, u64), String>{
        let mut liquidity_pool = match self.liquidity_pool.clone() {
            Some(liquidity_pool) => liquidity_pool,
            None =>  return  Err("provide_liquidity_lp: no liquidity pools".to_string()),
        };
        
        liquidity_pool.pool_1 += provided_amount;
        let ratio_amount:u64 = (provided_amount as f64 * liquidity_pool.liquidity_ratio as f64) as u64;
        liquidity_pool.pool_2 += provided_amount;
        self.supply += provided_amount + ratio_amount;
        if (provided_amount / ratio_amount) as f32 != liquidity_pool.liquidity_ratio {
            return Err("provide_liquidity_lp: liquidity not provided in the correct ratio".to_string())
        }

        let lp_utxo = format!("{}:0", txid);
        self.owners.insert(lp_utxo.to_string(), provided_amount + ratio_amount);
        liquidity_pool.k = liquidity_pool.pool_1 as u128 * liquidity_pool.pool_2 as u128;
        self.liquidity_pool = Some(liquidity_pool);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok((lp_utxo, provided_amount + ratio_amount));
    }

    pub fn swap_lp(&mut self, txid: &str, payload: &str, sender_contract_id: String, provided_amount: u64, quoted: u64, slipage_tolerance: f32)-> Result<u64, String>{
        let sender_pool_amount;
        let reciever_pool_amount;
        let mut liquidity_pool = match self.liquidity_pool.clone() {
            Some(liquidity_pool) => liquidity_pool,
            None =>  return  Err("swap_lp: no liquidity pools".to_string()),
        };

        if  sender_contract_id == liquidity_pool.contract_id_1 {
            sender_pool_amount = liquidity_pool.pool_1;
            reciever_pool_amount = liquidity_pool.pool_2;
        }else if sender_contract_id == liquidity_pool.contract_id_2  {
            sender_pool_amount = liquidity_pool.pool_2;
            reciever_pool_amount = liquidity_pool.pool_1;
        }else{
            return Err("swap_lp: contract_id not associated with this liquidity pool".to_string());
        }

        let fee_diff  = provided_amount as f64 * (1.0 - liquidity_pool.fee) as f64;
        let mut swap_amount = ((reciever_pool_amount as f64) - (liquidity_pool.k as f64 /(sender_pool_amount as f64 + fee_diff))) as u64;

        if swap_amount >  reciever_pool_amount {
            swap_amount = reciever_pool_amount;
        }

        if swap_amount.abs_diff(quoted) as f32 / quoted  as f32 > slipage_tolerance {
            if swap_amount >= quoted {
                println!("swap_lp: swap amount is above tolerence: {}", swap_amount);
                swap_amount = (quoted as f32 * (1.0 + slipage_tolerance)) as u64;        
            }else{
                println!("swap_lp: swap amount is below tolerence");
                return Ok(0);
            }
        }

        if  sender_contract_id == liquidity_pool.contract_id_1 {
            liquidity_pool.pool_1 += provided_amount;
            liquidity_pool.pool_2 -= swap_amount;
        }else{
            liquidity_pool.pool_2 += provided_amount;
            liquidity_pool.pool_1 -= swap_amount;
        }
        
        liquidity_pool.k = liquidity_pool.pool_2 as u128 * liquidity_pool.pool_1 as u128;
        liquidity_pool.swaps.insert(txid.to_string(), (provided_amount, swap_amount));
        self.liquidity_pool = Some(liquidity_pool);
        self.payloads.insert(txid.to_string(), payload.to_string());
        return Ok(swap_amount);
    }

    pub fn liquidate_postion_lp(&mut self, txid: &str, payload: &str, lp_utxos: &Vec<String>, claim_amount: u64, block_height: u64)-> Result<(u64, u64, String, u64, bool), String>{
        let mut liquidity_pool = match self.liquidity_pool.clone() {
            Some(liquidity_pool) => liquidity_pool,
            None =>  return  Err("liquidate_postion_lp: no liquidity pools".to_string()),
        };

        let mut total_tokens = 0;
        for lp_utxo in lp_utxos.clone() {
            if self.owners.contains_key(&lp_utxo) {
                total_tokens += self.owners[&lp_utxo];
            }
        }

        if total_tokens == 0 {
            return Err("liquidate_postion_lp: owner amount is zero".to_string());
        }
        
        if total_tokens >= claim_amount {    
            for utxo  in lp_utxos {
                self.owners.remove(utxo);
            };
    
            let change_utxo: String =  format!("{}:0", txid);
            let change_drip_present = self.transfer_drips(lp_utxos, block_height, &change_utxo);
            let change = total_tokens - claim_amount;
            let lp_token_ratio =  claim_amount as f64 / self.supply as f64;
            let token_1 =  (liquidity_pool.pool_1 as f64 * lp_token_ratio) as u64;
            let token_2 =  (liquidity_pool.pool_2 as f64 * lp_token_ratio) as u64;
            liquidity_pool.pool_1 -= token_1;
            liquidity_pool.pool_2 -= token_2;
    
    
            if change > 0 {
                self.owners.insert(change_utxo.to_string(), change);
            }
            
            liquidity_pool.k = liquidity_pool.pool_1 as u128 * liquidity_pool.pool_2 as u128;
            liquidity_pool.liquidations.insert(txid.to_string(), (token_1, token_2));
            self.liquidity_pool = Some(liquidity_pool);
            self.supply -= claim_amount;
            self.payloads.insert(txid.to_string(), payload.to_string());
            return Ok((token_1, token_2, change_utxo, change, change_drip_present));
        }else{
            return Err("liquidate_postion_lp: owner amount is less than the claim amount ".to_string());
        }


    }

    fn transfer_drips(&mut self, utxos: &Vec<String>, block_height: u64, change_utxo: &String) -> bool{
        let mut drips = match self.drips.clone() {
            Some(drips) => drips,
            None => HashMap::new(),
        };

        let mut change_drip_present = false;
        let mut new_drips: Vec<Drip> = Vec::new();
        for utxo in utxos.clone() {
            if self.owners.contains_key(&utxo.clone()) {
                if let Some(old_drips) = drips.get(&utxo) {
                    for drip in old_drips {
                        let new_drip = Drip {
                            block_end: drip.block_end.clone(),
                            drip_amount: drip.drip_amount.clone(),
                            amount: drip.amount.clone() - (block_height - drip.start_block) * drip.drip_amount,
                            start_block: block_height,
                            last_block_dripped: block_height
                        };

                        new_drips.push(new_drip.clone());
                    }
                    
                    // Remove the old drip from the vector
                    drips.remove(&utxo);
                }
            }
        }

        if !new_drips.is_empty() {    
            change_drip_present = true;
            drips.insert(change_utxo.clone(), new_drips);
        }

        self.drips = Some(drips);
        return change_drip_present;
    }
}

#[derive(Debug, Deserialize, Default, Serialize, Clone,PartialEq)]
pub struct Drip {
    pub block_end: u64,
    pub drip_amount: u64,
    pub amount: u64,
    pub start_block: u64,
    pub last_block_dripped: u64
}

#[derive(Debug, Deserialize, Default, Serialize, Clone,PartialEq)]
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

#[derive(Debug, Deserialize, Default, Serialize, Clone,PartialEq)]
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

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct Listing {
    pub list_utxo: String,
    pub list_amt: u64,
    pub price: u64,
    pub rec_addr: String,
    pub change_utxo:String,
    pub valid_bid_block: Option<i32>
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct Bid {
    pub bid_price: u64,
    pub bid_amount: u64,
    pub order_id: String,
    pub fulfill_tx: String,
    pub accept_tx: String,
    pub reseved_utxo:String,
    pub fullfilment_utxos: Vec<String>
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LiquidityProvider {
    pub provided_block_height: u32,
    pub lp_tokens: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LiquidityPool {
    pub contract_id_1: String,
    pub contract_id_2: String,
    pub pool_1: u64,
    pub pool_2: u64,
    pub fee: f32,
    pub k: u128,
    pub liquidity_ratio: f32,
    pub swaps: HashMap<String, (u64,u64)>,
    pub liquidations: HashMap<String, (u64,u64)>
}