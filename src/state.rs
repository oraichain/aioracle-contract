use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Order};

use cw_storage_plus::{
    Bound, Bounder, Index, IndexList, IndexedMap, Item, MultiIndex, UniqueIndex,
};

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
    pub max_req_threshold: u64,
}

#[cw_serde]
pub struct Request {
    /// Owner If None set, contract is frozen.
    pub requester: Addr,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub input: Option<Binary>,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u64> = Item::new(LATEST_STAGE_KEY);

pub const EXECUTORS_INDEX_PREFIX: &str = "executors_index";
pub const EXECUTORS_INDEX: Item<u64> = Item::new(EXECUTORS_INDEX_PREFIX);

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
