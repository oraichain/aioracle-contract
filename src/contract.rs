use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, CanonicalAddr, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128,
};

use cw2::set_contract_version;

use sha2::Digest;
use std::convert::TryInto;
use std::ops::Mul;

use crate::error::ContractError;
use crate::executors::{query_executor_size, query_executors, remove_executors, store_executors};

use crate::msg::{
    AddServiceMsg, ExecuteMsg, InstantiateMsg, LatestStageResponse, MigrateMsg, QueryMsg,
    RequestResponse, ServiceInfoResponse, UpdateConfigMsg, UpdateServiceMsg,
};
use crate::state::{
    config_read, config_save, config_update, get_range_params, latest_stage_read,
    latest_stage_save, latest_stage_update, read_executor, read_service_info, read_service_infos,
    remove_service_info, requests, store_service_info, Config, Request, ServiceInfo,
};
pub const MAXIMUM_REQ_THRESHOLD: u64 = 67;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aioracle-v2";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
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
    // let mut executor_index = 0;
    if let Some(executors) = msg.executors {
        store_executors(
            deps.storage,
            executors
                .into_iter()
                .map(|ex| Ok(deps.api.addr_canonicalize(&ex)?))
                .collect::<StdResult<Vec<CanonicalAddr>>>()?,
        );
    } else {
        store_executors(deps.storage, vec![])
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { update_config_msg } => {
            execute_update_config(deps, env, info, update_config_msg)
        }
        ExecuteMsg::RegisterMerkleRoot {
            stage,
            merkle_root,
            executors,
        } => execute_register_merkle_root(deps, env, info, stage, merkle_root, executors),
        ExecuteMsg::Request {
            service,
            input,
            threshold,
        } => handle_request(deps, info, env, service, input, threshold),
        ExecuteMsg::AddService(service_msg) => handle_add_service(deps, info, service_msg),
        ExecuteMsg::UpdateService(service_msg) => handle_update_service(deps, info, service_msg),
        ExecuteMsg::DeleteService { service_name } => {
            handle_delete_service(deps, info, service_name)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    // once we have "migrated", set the new version and return success
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attributes(vec![
        attr("new_contract_name", CONTRACT_NAME),
        attr("new_contract_version", CONTRACT_VERSION),
    ]))
}

pub fn handle_add_service(
    deps: DepsMut,
    info: MessageInfo,
    service_msg: AddServiceMsg,
) -> Result<Response, ContractError> {
    let service_info = read_service_info(deps.storage, service_msg.service_name.as_bytes()).ok();
    if service_info.is_some() {
        return Err(ContractError::Unauthorized {});
    }
    store_service_info(
        deps.storage,
        service_msg.service_name.as_bytes(),
        &ServiceInfo {
            owner: info.sender,
            service: service_msg.service,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "add_service"),
        attr("service_name", service_msg.service_name),
    ]))
}

pub fn handle_update_service(
    deps: DepsMut,
    info: MessageInfo,
    service_msg: UpdateServiceMsg,
) -> Result<Response, ContractError> {
    let service_info = read_service_info(deps.storage, service_msg.service_name.as_bytes()).ok();
    if let Some(mut service_info) = service_info {
        if service_info.owner.ne(&info.sender) {
            return Err(ContractError::Unauthorized {});
        }
        if let Some(new_owner) = service_msg.new_owner {
            service_info.owner = deps.api.addr_validate(&new_owner)?;
        }
        if let Some(dsources) = service_msg.dsources {
            service_info.service.dsources = dsources;
        }
        if let Some(tcases) = service_msg.tcases {
            service_info.service.tcases = tcases;
        }
        if let Some(oscript_url) = service_msg.oscript_url {
            service_info.service.oscript_url = oscript_url;
        }
        store_service_info(
            deps.storage,
            service_msg.service_name.as_bytes(),
            &service_info,
        )?;
        return Ok(Response::new().add_attributes(vec![
            attr("action", "update_service"),
            attr("service_name", service_msg.service_name),
        ]));
    }

    Err(ContractError::ServiceNotFound {})
}

pub fn handle_delete_service(
    deps: DepsMut,
    info: MessageInfo,
    service_name: String,
) -> Result<Response, ContractError> {
    let service_info = read_service_info(deps.storage, service_name.as_bytes()).ok();
    if let Some(service_info) = service_info {
        if service_info.owner.ne(&info.sender) {
            return Err(ContractError::Unauthorized {});
        }
        remove_service_info(deps.storage, service_name.as_bytes());
        return Ok(Response::new().add_attributes(vec![
            attr("action", "delete_service"),
            attr("service_name", service_name),
        ]));
    }

    return Err(ContractError::Unauthorized {});
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
        new_max_req_threshold,
        ..
    } = update_config_msg;
    let cfg = config_read(deps.storage)?;
    let owner = cfg.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    config_update(deps.storage, new_owner, new_max_req_threshold)?;

    if let Some(executors) = new_executors {
        store_executors(
            deps.storage,
            executors
                .into_iter()
                .map(|executor| deps.api.addr_canonicalize(&executor))
                .collect::<StdResult<Vec<CanonicalAddr>>>()?,
        );
    }
    if let Some(executors) = old_executors {
        remove_executors(
            deps.storage,
            executors
                .into_iter()
                .map(|executor| deps.api.addr_canonicalize(&executor))
                .collect::<StdResult<Vec<CanonicalAddr>>>()?,
        );
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
) -> Result<Response, ContractError> {
    let stage = latest_stage_update(deps.storage)?;
    let Config {
        max_req_threshold, ..
    } = config_read(deps.storage)?;

    // this will keep track of the executor list of the request
    let current_size = query_executor_size(deps.as_ref());

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
        stage,
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

    let Request { merkle_root, .. } = requests().load(deps.storage, stage)?;
    if merkle_root.ne("") {
        return Err(ContractError::AlreadyFinished {});
    }

    // if merkle root empty then update new
    requests().update(deps.storage, stage, |request| {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetExecutors {
            start,
            end,
            order,
            limit,
        } => to_binary(&query_executors(deps, start, end, order, limit)?),
        QueryMsg::CheckExecutorInList { address } => {
            let result = read_executor(deps.storage, deps.api.addr_canonicalize(&address)?);
            to_binary(&result.is_some())
        }
        QueryMsg::GetExecutorSize {} => to_binary(&query_executor_size(deps)),
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
        QueryMsg::GetService { service_name } => {
            let service = read_service_info(deps.storage, service_name.as_bytes())?;
            to_binary(&service)
        }
        QueryMsg::GetServices {
            start,
            end,
            order,
            limit,
        } => to_binary(&query_services(deps, start, end, order, limit)?),
    }
}

pub fn verify_data(
    deps: Deps,
    stage: u64,
    data: Binary,
    proofs: Option<Vec<String>>,
) -> StdResult<bool> {
    let Request { merkle_root, .. } = requests().load(deps.storage, stage)?;
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
    let request = requests().load(deps.storage, stage)?;
    Ok(request)
}

fn parse_request<'a>(item: StdResult<(u64, Request)>) -> StdResult<RequestResponse> {
    item.and_then(|(id, request)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        Ok(RequestResponse {
            stage: id,
            requester: request.requester,
            request_height: request.request_height,
            submit_merkle_height: request.submit_merkle_height,
            merkle_root: request.merkle_root,
            threshold: request.threshold,
            service: request.service,
            input: request.input,
        })
    })
}

pub fn query_requests(
    deps: Deps,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = get_range_params(offset, limit, order);
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
    let (limit, min, max, order_enum) = get_range_params(offset, limit, order);
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .service
        .prefix(service.as_bytes().to_vec())
        .range(deps.storage, min, max, order_enum)
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
    let (limit, min, max, order_enum) = get_range_params(offset, limit, order);
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .merkle_root
        .prefix(merkle_root.as_bytes().to_vec())
        .range(deps.storage, min, max, order_enum)
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

pub fn query_services(
    deps: Deps,
    start: Option<String>,
    end: Option<String>,
    order: Option<u8>,
    limit: Option<u8>,
) -> StdResult<Vec<ServiceInfoResponse>> {
    read_service_infos(
        deps.storage,
        start.map(|start| String::into_bytes(start)).as_deref(),
        end.map(|end| String::into_bytes(end)).as_deref(),
        order,
        limit,
    )
}
