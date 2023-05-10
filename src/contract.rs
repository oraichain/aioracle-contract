use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use cw2::set_contract_version;

use sha2::Digest;
use std::convert::TryInto;
use std::ops::Mul;

use crate::error::ContractError;
use crate::executors::{
    query_executor, query_executor_size, query_executors, query_executors_by_index,
    remove_executors, save_executors, update_executors,
};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, LatestStageResponse, MigrateMsg, QueryMsg, RequestResponse,
    UpdateConfigMsg,
};
use crate::state::{
    config_read, config_save, config_update, get_range_params, latest_stage_read,
    latest_stage_save, latest_stage_update, requests, Config, Request,
};
pub const MAXIMUM_REQ_THRESHOLD: u64 = 67;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aioracle-v2";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let owner = msg.owner.unwrap_or(info.sender);

    let config = Config {
        owner,
        max_req_threshold: MAXIMUM_REQ_THRESHOLD,
    };
    config_save(deps.storage, &config)?;

    let stage = 0;
    latest_stage_save(deps.storage, &stage)?;

    // first nonce
    let mut executor_index = 0;
    save_executors(deps.storage, vec![])?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { update_config_msg } => {
            execute_update_config(deps, env, info, update_config_msg)
        }
        // ExecuteMsg::ToggleExecutorActiveness { pubkey } => {
        //     toggle_executor_activeness(deps, info,, pubkey)
        // }
        ExecuteMsg::RegisterMerkleRoot {
            stage,
            merkle_root,
            executors,
        } => execute_register_merkle_root(deps, env, info, stage, merkle_root, executors),
        ExecuteMsg::Request {
            service,
            input,
            threshold,
            preference_executor_fee,
        } => handle_request(
            deps,
            info,
            env,
            service,
            input,
            threshold,
            preference_executor_fee,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<Response> {
    // once we have "migrated", set the new version and return success
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attributes(vec![
        attr("new_contract_name", CONTRACT_NAME),
        attr("new_contract_version", CONTRACT_VERSION),
    ]))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    update_config_msg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    // authorize owner
    let UpdateConfigMsg {
        new_owner,
        new_executors,
        old_executors,
        new_checkpoint,
        new_checkpoint_threshold,
        new_max_req_threshold,
    } = update_config_msg;
    let cfg = config_read(deps.storage)?;
    let owner = cfg.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    config_update(deps.storage, new_owner, new_max_req_threshold)?;

    if let Some(executors) = new_executors {
        update_executors(deps.storage, executors)?;
    }
    if let Some(executors) = old_executors {
        remove_executors(deps.storage, executors)?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn handle_request(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    service: String,
    input: Option<Binary>,
    threshold: u64,
    preference_executor_fee: Coin,
) -> Result<Response, ContractError> {
    let stage = latest_stage_update(deps.storage)?;
    let Config {
        max_req_threshold, ..
    } = config_read(deps.storage)?;

    // this will keep track of the executor list of the request
    let current_size = query_executor_size(deps.as_ref())?;

    if Uint128::from(current_size)
        .mul(Decimal::from_ratio(
            Uint128::from(max_req_threshold).u128(),
            100u128,
        ))
        .lt(&Uint128::from(threshold))
    {
        return Err(ContractError::InvalidThreshold {});
    }

    requests().save(
        deps.storage,
        &stage.to_be_bytes(),
        &crate::state::Request {
            requester: info.sender.clone(),
            request_height: env.block.height,
            submit_merkle_height: 0u64,
            merkle_root: String::from(""),
            threshold,
            service: service.clone(),
            input,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "handle_request"),
        attr("stage", stage.to_string()),
        attr("threshold", threshold.to_string()),
        attr("service", service),
    ]))
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u64,
    mroot: String,
    executors: Vec<String>,
) -> Result<Response, ContractError> {
    let Config { owner, .. } = config_read(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // validate executor list for client verification
    for executor in executors {
        deps.api.addr_validate(&executor)?;
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(mroot.to_string(), &mut root_buf)?;

    let Request { merkle_root, .. } = requests().load(deps.storage, &stage.to_be_bytes())?;
    if merkle_root.ne("") {
        return Err(ContractError::AlreadyFinished {});
    }

    // if merkle root empty then update new
    let request = requests().update(deps.storage, &stage.to_be_bytes(), |request| {
        if let Some(mut request) = request {
            request.merkle_root = mroot.clone();
            request.submit_merkle_height = env.block.height;
            {
                return Ok(request);
            }
        }
        Err(StdError::generic_err("Invalid request empty"))
    })?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("current_stage", stage.to_string()),
        attr("merkle_root", mroot),
    ]))
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetExecutors {
            offset,
            limit,
            order,
        } => to_binary(&query_executors(deps, offset, limit, order)?),
        QueryMsg::GetExecutorsByIndex {
            offset,
            limit,
            order,
        } => to_binary(&query_executors_by_index(deps, offset, limit, order)?),
        QueryMsg::GetExecutor { pubkey } => to_binary(&query_executor(deps, pubkey)?),
        QueryMsg::GetExecutorSize {} => to_binary(&query_executor_size(deps)?),
        QueryMsg::GetRequest { stage } => to_binary(&query_request(deps, stage)?),
        QueryMsg::GetRequests {
            offset,
            limit,
            order,
        } => to_binary(&query_requests(deps, offset, limit, order)?),
        QueryMsg::GetRequestsByService {
            service,
            offset,
            limit,
            order,
        } => to_binary(&query_requests_by_service(
            deps, service, offset, limit, order,
        )?),
        QueryMsg::GetRequestsByMerkleRoot {
            merkle_root,
            offset,
            limit,
            order,
        } => to_binary(&query_requests_by_merkle_root(
            deps,
            merkle_root,
            offset,
            limit,
            order,
        )?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::VerifyData { stage, data, proof } => {
            to_binary(&verify_data(deps, stage, data, proof)?)
        }
    }
}

pub fn verify_data(
    deps: Deps,
    stage: u64,
    data: Binary,
    proofs: Option<Vec<String>>,
) -> StdResult<bool> {
    let Request { merkle_root, .. } = requests().load(deps.storage, &stage.to_be_bytes())?;
    if merkle_root.is_empty() {
        return Err(StdError::generic_err(
            "No merkle root found for this request",
        ));
    }
    let mut final_proofs: Vec<String> = vec![];
    if let Some(proofs) = proofs {
        final_proofs = proofs;
    }

    let hash = sha2::Sha256::digest(data.as_slice())
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("wrong length"))?;

    let hash = final_proofs.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)
            .map_err(|_| StdError::generic_err("error decoding"))?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| StdError::generic_err("wrong length"))
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)
        .map_err(|_| StdError::generic_err("error decoding"))?;
    let mut verified = false;
    if root_buf == hash {
        verified = true;
    }
    Ok(verified)
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    config_read(deps.storage)
}

// ============================== Query Handlers ==============================

pub fn query_request(deps: Deps, stage: u64) -> StdResult<Request> {
    let request = requests().load(deps.storage, &stage.to_be_bytes())?;
    Ok(request)
}

fn parse_request<'a>(item: StdResult<KV<Request>>) -> StdResult<RequestResponse> {
    item.and_then(|(k, request)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse offering key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(RequestResponse {
            stage: id,
            requester: request.requester,
            request_height: request.request_height,
            submit_merkle_height: request.submit_merkle_height,
            merkle_root: request.merkle_root,
            threshold: request.threshold,
            service: request.service,
        })
    })
}

pub fn query_requests(
    deps: Deps,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = get_range_params(limit, offset, order);
    let requests: StdResult<Vec<RequestResponse>> = requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(requests?)
}

pub fn query_requests_by_service(
    deps: Deps,
    service: String,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = get_range_params(
        offset.map(|v| v.to_be_bytes().to_vec().as_slice()),
        limit,
        order,
    );
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .service
        .items(deps.storage, service.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(request_responses?)
}

pub fn query_requests_by_merkle_root(
    deps: Deps,
    merkle_root: String,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = get_range_params(limit, offset, order);
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .merkle_root
        .items(deps.storage, merkle_root.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(request_responses?)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = latest_stage_read(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}
