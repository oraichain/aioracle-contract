use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, CanonicalAddr, Order, StdResult, Storage};

use cosmwasm_storage::{bucket, bucket_read, prefixed, prefixed_read, singleton, singleton_read};
use cw_storage_plus::{Bound, Bounder, Index, IndexList, IndexedMap, MultiIndex};

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

#[cw_serde]
pub struct DataSourceState {
    pub language: String,
    pub script_url: String,
    pub parameters: Vec<Binary>,
}

#[cw_serde]
pub struct TestCaseState {
    pub input: Binary,
    pub expected_output: Binary,
}

#[cw_serde]
pub struct Service {
    pub dsources: Vec<DataSourceState>,
    pub tcases: Vec<TestCaseState>,
    pub oscript_url: String,
}

#[cw_serde]
pub struct ServiceInfo {
    pub owner: Addr,
    pub service: Service,
}

pub fn store_service_info(
    storage: &mut dyn Storage,
    service_name: &[u8],
    service_info: &ServiceInfo,
) -> StdResult<()> {
    bucket(storage, PREFIX_SERVICE_INFO).save(service_name, service_info)
}

pub fn read_service_info(storage: &dyn Storage, service_name: &[u8]) -> StdResult<ServiceInfo> {
    bucket_read(storage, PREFIX_SERVICE_INFO).load(service_name)
}

pub fn remove_service_info(storage: &mut dyn Storage, service_name: &[u8]) {
    bucket::<ServiceInfo>(storage, PREFIX_SERVICE_INFO).remove(service_name)
}

pub fn config_save(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn store_executor(storage: &mut dyn Storage, executor: CanonicalAddr) {
    prefixed(storage, PREFIX_EXECUTOR).set(executor.as_slice(), &[])
}

pub fn read_executor(storage: &dyn Storage, executor: CanonicalAddr) -> Option<Vec<u8>> {
    prefixed_read(storage, PREFIX_EXECUTOR).get(executor.as_slice())
}

pub fn executor_prefixes(storage: &dyn Storage) -> cosmwasm_storage::ReadonlyPrefixedStorage {
    prefixed_read(storage, PREFIX_EXECUTOR)
}

pub fn remove_executor(storage: &mut dyn Storage, executor: CanonicalAddr) {
    prefixed(storage, PREFIX_EXECUTOR).remove(executor.as_slice())
}

pub fn config_update(
    storage: &mut dyn Storage,
    new_owner: Option<Addr>,
    new_max_req_threshold: Option<u64>,
) -> StdResult<Config> {
    let mut config = config_read(storage)?;
    let mut need_update = false;

    if let Some(new_owner) = new_owner {
        config.owner = new_owner;
        need_update = true;
    }
    if let Some(max_req_threshold) = new_max_req_threshold {
        config.max_req_threshold = max_req_threshold;
        need_update = true;
    }
    if need_update {
        config_save(storage, &config)?;
    }
    Ok(config)
}

pub fn config_read(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn latest_stage_read(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LATEST_STAGE).load()
}

pub fn latest_stage_save(storage: &mut dyn Storage, latest_stage: &u64) -> StdResult<()> {
    singleton(storage, KEY_LATEST_STAGE).save(latest_stage)
}

pub fn latest_stage_update(storage: &mut dyn Storage) -> StdResult<u64> {
    singleton(storage, KEY_LATEST_STAGE).update(|latest_stage: u64| Ok(latest_stage + 1))
}

// indexes requests
// for structures
pub struct RequestIndexes<'a> {
    pub service: MultiIndex<'a, Vec<u8>, Request, u64>,
    pub merkle_root: MultiIndex<'a, Vec<u8>, Request, u64>,
    pub requester: MultiIndex<'a, Vec<u8>, Request, u64>,
}

impl<'a> IndexList<Request> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Request>> + '_> {
        let v: Vec<&dyn Index<Request>> = vec![&self.service, &self.merkle_root, &self.requester];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn requests<'a>() -> IndexedMap<'a, u64, Request, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        service: MultiIndex::new(
            |_pk, d| d.service.as_bytes().to_vec(),
            "requests",
            "requests_service",
        ),
        merkle_root: MultiIndex::new(
            |_pk, d| d.merkle_root.as_bytes().to_vec(),
            "requests",
            "requests_merkle_root",
        ),
        requester: MultiIndex::new(
            |_pk, d| d.requester.as_bytes().to_vec(),
            "requests",
            "requests_requester",
        ),
    };
    IndexedMap::new("requests", indexes)
}

// settings for pagination
pub const MAX_LIMIT: u8 = 50;
pub const DEFAULT_LIMIT: u8 = 20;

pub fn get_range_params<'a, T: Bounder<'a>>(
    offset: Option<T>,
    limit: Option<u8>,
    order: Option<u8>,
) -> (usize, Option<Bound<'a, T>>, Option<Bound<'a, T>>, Order) {
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

pub static KEY_CONFIG: &[u8] = b"config";
pub static KEY_LATEST_STAGE: &[u8] = b"latest_stage";
pub static PREFIX_SERVICE_INFO: &[u8] = b"service_info";
pub static PREFIX_EXECUTOR: &[u8] = b"executor";
