use cosmwasm_std::{CanonicalAddr, Deps, Order, StdResult, Storage};

use crate::state::{
    executor_prefixes, match_order, remove_executor, store_executor, DEFAULT_LIMIT,
};

pub fn store_executors(storage: &mut dyn Storage, executors: Vec<CanonicalAddr>) {
    for executor in executors {
        store_executor(storage, executor)
    }
}

pub fn remove_executors(storage: &mut dyn Storage, executors: Vec<CanonicalAddr>) {
    for executor in executors {
        remove_executor(storage, executor)
    }
}

// query functions

pub fn query_executors(
    deps: Deps,
    start: Option<String>,
    end: Option<String>,
    order: Option<u8>,
    limit: Option<u8>,
) -> StdResult<Vec<String>> {
    let executor_prefixes = executor_prefixes(deps.storage);
    let executors: StdResult<Vec<String>> = executor_prefixes
        .range(
            start.map(|start| String::into_bytes(start)).as_deref(),
            end.map(|start| String::into_bytes(start)).as_deref(),
            match_order(order),
        )
        .take(limit.unwrap_or(DEFAULT_LIMIT) as usize)
        .map(|executor| {
            deps.api
                .addr_humanize(&CanonicalAddr::from(executor.0))
                .map(|executor| executor.to_string())
        })
        .collect();
    Ok(executors?)
}

pub fn query_executor_size(deps: Deps) -> u64 {
    executor_prefixes(deps.storage)
        .range(None, None, Order::Ascending)
        .count() as u64
}
