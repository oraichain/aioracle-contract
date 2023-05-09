use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Coin, Order, Uint128};

use cw_storage_plus::{
    Bound, Bounder, Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex,
};

pub type Reward = (Addr, String, Uint128);

#[cw_serde]
pub struct Executor {
    /// Owner If None set, contract is frozen.
    pub pubkey: Binary,
    pub is_active: bool,
    pub executing_power: u64,
    pub index: u64,
    pub left_block: Option<u64>,
}

#[cw_serde]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Addr,
    pub service_addr: Addr,
    pub contract_fee: Coin,
    /// this threshold is to update the checkpoint stage when current previous checkpoint +
    pub checkpoint_threshold: u64,
    pub max_req_threshold: u64,
    pub trusting_period: u64,
    pub slashing_amount: u64,
    pub denom: String,
    pub pending_period: u64,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Contracts {
    pub dsources: Vec<Addr>,
    pub tcases: Vec<Addr>,
    pub oscript: Addr,
}

#[cw_serde]
pub struct Request {
    /// Owner If None set, contract is frozen.
    pub requester: Addr,
    pub preference_executor_fee: Coin,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub input: Option<String>,
    pub rewards: Vec<Reward>,
}

#[cw_serde]
pub struct TrustingPool {
    /// Owner If None set, contract is frozen.
    pub amount_coin: Coin,
    pub withdraw_amount_coin: Coin,
    pub withdraw_height: u64,
}

pub const CONFIG_KEY: &str = "config_v3";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u64> = Item::new(LATEST_STAGE_KEY);

pub const CHECKPOINT_STAGE_KEY: &str = "checkpoint";
pub const CHECKPOINT: Item<u64> = Item::new(CHECKPOINT_STAGE_KEY);

pub const CLAIM_PREFIX: &str = "claim";

// key: executor in base64 string + stage in string
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);

pub const EVIDENCE_PREFIX: &str = "evidence";

// key: executor in base64 string + stage in string
pub const EVIDENCES: Map<&[u8], bool> = Map::new(EVIDENCE_PREFIX);

// pub const EXECUTORS_PREFIX: &str = "executors";
// pub const EXECUTORS: Map<&[u8], bool> = Map::new(EXECUTORS_PREFIX);

pub const EXECUTORS_INDEX_PREFIX: &str = "executors_index";
pub const EXECUTORS_INDEX: Item<u64> = Item::new(EXECUTORS_INDEX_PREFIX);

pub const CONTRACT_FEES_INDEX: &str = "contract_fees_index";
pub const CONTRACT_FEES: Item<Coin> = Item::new(CONTRACT_FEES_INDEX);

pub const EXECUTORS_TRUSTING_POOL_PREFIX: &str = "executors_trusting_pool_v2";
pub const EXECUTORS_TRUSTING_POOL: Map<&[u8], TrustingPool> =
    Map::new(EXECUTORS_TRUSTING_POOL_PREFIX);

// indexes requests
// for structures
pub struct RequestIndexes<'a> {
    pub service: MultiIndex<'a, &'a [u8], Request, u64>,
    pub merkle_root: MultiIndex<'a, &'a [u8], Request, u64>,
    pub requester: MultiIndex<'a, &'a [u8], Request, u64>,
}

impl<'a> IndexList<Request> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Request>> + '_> {
        let v: Vec<&dyn Index<Request>> = vec![&self.service, &self.merkle_root, &self.requester];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn requests<'a>() -> IndexedMap<'a, &'a [u8], Request, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        service: MultiIndex::new(
            |_pk, d| d.service.as_bytes(),
            "requests",
            "requests_service",
        ),
        merkle_root: MultiIndex::new(
            |_pk, d| d.merkle_root.as_bytes(),
            "requests",
            "requests_merkle_root",
        ),
        requester: MultiIndex::new(
            |_pk, d| d.requester.as_bytes(),
            "requests",
            "requests_requester",
        ),
    };
    IndexedMap::new("requests", indexes)
}

// index for executors

pub struct ExecutorIndexes<'a> {
    pub is_active: MultiIndex<'a, &'a [u8], Executor, u64>,
    pub index: UniqueIndex<'a, &'a [u8], Executor>,
}

impl<'a> IndexList<Executor> for ExecutorIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Executor>> + '_> {
        let v: Vec<&dyn Index<Executor>> = vec![&self.is_active, &self.index];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn executors_map<'a>() -> IndexedMap<'a, &'a [u8], Executor, ExecutorIndexes<'a>> {
    let indexes = ExecutorIndexes {
        is_active: MultiIndex::new(
            |_pk, d| if d.is_active { &[1u8] } else { &[0u8] },
            "executors",
            "executors_is_active",
        ),
        index: UniqueIndex::new(|d| &d.index.to_be_bytes(), "index"),
    };
    IndexedMap::new("executors", indexes)
}

// settings for pagination
pub const MAX_LIMIT: u8 = 50;
pub const DEFAULT_LIMIT: u8 = 20;

pub fn get_range_params<'a>(
    offset: Option<&[u8]>,
    limit: Option<u8>,
    order: Option<u8>,
) -> (
    usize,
    Option<Bound<'a, &'a [u8]>>,
    Option<Bound<'a, &'a [u8]>>,
    Order,
) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
    let mut min = None;
    let mut max = None;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }
    let offset_value = match offset {
        Some(value) => value.exclusive_bound(),
        None => None,
    };

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max, order_enum)
}
