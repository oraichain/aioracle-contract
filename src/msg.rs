use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin};

use crate::state::{DataSourceState, Service, TestCaseState};

#[cw_serde]
pub struct ServiceMsg {
    pub service: String,
}

#[cw_serde]
pub struct ServiceFeesMsg {
    pub addr: Addr,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner if none set to info.sender.
    pub owner: Option<Addr>,
    pub executors: Option<Vec<String>>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        update_config_msg: UpdateConfigMsg,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        stage: u64,
        merkle_root: String,
        executors: Vec<String>,
    },
    Request {
        service: String,
        input: Option<Binary>,
        threshold: u64,
        preference_executor_fee: Coin,
    },
    AddService(AddServiceMsg),
    UpdateService(UpdateServiceMsg),
    DeleteService {
        service_name: String,
    },
}

#[cw_serde]
pub struct UpdateServiceMsg {
    pub service_name: String,
    pub new_owner: Option<String>,
    pub dsources: Option<Vec<DataSourceState>>,
    pub tcases: Option<Vec<TestCaseState>>,
    pub oscript_url: Option<String>,
}

#[cw_serde]
pub struct AddServiceMsg {
    pub service_name: String,
    pub service: Service,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(Vec<String>)]
    GetExecutors {
        start: Option<String>, // ordered by keys, so we pass in executor addresses
        end: Option<String>,
        order: Option<u8>,
        limit: Option<u8>,
    },
    #[returns(bool)]
    CheckExecutorInList { address: String },
    #[returns(u64)]
    GetExecutorSize {},
    #[returns(RequestResponse)]
    GetRequest { stage: u64 },
    #[returns(Vec<RequestResponse>)]
    GetRequests {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(Vec<RequestResponse>)]
    GetRequestsByService {
        service: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(Vec<RequestResponse>)]
    GetRequestsByMerkleRoot {
        merkle_root: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(LatestStageResponse)]
    LatestStage {},
    #[returns(bool)]
    VerifyData {
        stage: u64,
        data: Binary,
        proof: Option<Vec<String>>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Option<String>,
}

#[cw_serde]
pub struct ExecutorsResponse {
    pub pubkey: Binary,
    pub is_acitve: bool,
}

#[cw_serde]
pub struct RequestResponse {
    pub stage: u64,
    /// Owner If None set, contract is frozen.
    pub requester: Addr,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    /// Owner If None set, contract is frozen.
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
}

#[cw_serde]
pub struct LatestStageResponse {
    pub latest_stage: u64,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub new_owner: Option<Addr>,
    pub new_executors: Option<Vec<String>>,
    pub old_executors: Option<Vec<String>>,
    pub new_checkpoint: Option<u64>,
    pub new_checkpoint_threshold: Option<u64>,
    pub new_max_req_threshold: Option<u64>,
}
