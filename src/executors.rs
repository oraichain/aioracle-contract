use cosmwasm_std::{Binary, Deps, Order, StdResult, Storage};

use crate::state::{executors_map, get_range_params, Executor};

pub fn save_executors(storage: &mut dyn Storage, executors: Vec<Executor>) -> StdResult<()> {
    for executor in executors {
        executors_map().save(storage, executor.pubkey.to_vec(), &executor)?
    }
    Ok(())
}

pub fn remove_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    for executor in executors {
        let executor_option = executors_map().may_load(storage, executor.to_vec())?;
        if let Some(executor) = executor_option {
            executors_map().save(
                storage,
                executor.pubkey.clone().to_vec(),
                &Executor {
                    is_active: false,
                    ..executor
                },
            )?;
        } else {
            continue;
        }
    }
    Ok(())
}

pub fn update_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    let final_executors = executors
        .into_iter()
        .map(|executor| -> Executor {
            let old_executor_option = executors_map()
                .may_load(storage, executor.to_vec())
                .unwrap_or(None);
            // if executor exist then we dont increment executor index, reuse all config, only turn is active to true
            if let Some(old_executor) = old_executor_option {
                return Executor {
                    is_active: true,
                    ..old_executor
                };
            }
            // otherwise, we return new executor data
            let final_executor: Executor = Executor {
                pubkey: executor,
                is_active: true,
                left_block: None,
            };
            final_executor
        })
        .collect();
    save_executors(storage, final_executors)?;
    Ok(())
}

// query functions

pub fn query_executor(deps: Deps, pubkey: Binary) -> StdResult<Executor> {
    Ok(executors_map().load(deps.storage, pubkey.to_vec())?)
}

pub fn query_executors(
    deps: Deps,
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Executor>> {
    let (limit, min, max, order_enum) = get_range_params(offset.map(|o| o.to_vec()), limit, order);

    let res: StdResult<Vec<Executor>> = executors_map()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(_, executor)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(executor)
            })
        })
        .collect();
    res
}

pub fn query_executors_by_index(
    deps: Deps,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Executor>> {
    let (limit, min, max, order_enum) =
        get_range_params(offset.map(|v| v.to_be_bytes().to_vec()), limit, order);

    let res: StdResult<Vec<Executor>> = executors_map()
        .idx
        .index
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(_, executor)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(executor)
            })
        })
        .collect();
    res
}

pub fn query_executor_size(deps: Deps) -> StdResult<u64> {
    let executor_count = executors_map()
        .range(deps.storage, None, None, Order::Ascending)
        .count();
    Ok(executor_count as u64)
}
