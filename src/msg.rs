use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Coin, Uint128};

use crate::state::{Reward, TrustingPool};

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
    pub service_addr: Addr,
    pub contract_fee: Coin,
    pub executors: Vec<Binary>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        update_config_msg: UpdateConfigMsg,
    },
    // ToggleExecutorActiveness {
    //     pubkey: Binary,
    // },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        stage: u64,
        merkle_root: String,
        executors: Vec<Binary>,
    },
    Request {
        service: String,
        input: Option<String>,
        threshold: u64,
        preference_executor_fee: Coin,
    },
    // ClaimReward {
    //     stage: u64,
    //     report: Binary,
    //     proof: Option<Vec<String>>,
    // },
    WithdrawFees {
        amount: Uint128,
        denom: String,
    },
    PrepareWithdrawPool {
        pubkey: Binary,
    },
    ExecutorJoin {
        executor: Binary,
    },
    ExecutorLeave {
        executor: Binary,
    },
    SubmitEvidence {
        stage: u64,
        report: Binary,
        proof: Option<Vec<String>>,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    GetExecutors {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetExecutorsByIndex {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetExecutor {
        pubkey: Binary,
    },
    GetExecutorSize {},
    GetRequest {
        stage: u64,
    },
    GetRequests {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRequestsByService {
        service: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRequestsByMerkleRoot {
        merkle_root: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    LatestStage {},
    StageInfo {},
    GetServiceContracts {
        stage: u64,
    },
    IsClaimed {
        stage: u64,
        executor: Binary,
    },
    VerifyData {
        stage: u64,
        data: Binary,
        proof: Option<Vec<String>>,
    },
    GetServiceFees {
        service: String,
    },
    GetBoundExecutorFee {},
    GetParticipantFee {
        pubkey: Binary,
    },
    GetTrustingPool {
        pubkey: Binary,
    },
    GetTrustingPools {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
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
pub struct TrustingPoolResponse {
    pub pubkey: Binary,
    pub current_height: u64,
    pub trusting_period: u64,
    pub trusting_pool: TrustingPool,
}

#[cw_serde]
pub struct StageInfo {
    pub latest_stage: u64,
    pub checkpoint: u64,
    pub checkpoint_threshold: u64,
}

#[cw_serde]
pub struct Report {
    pub executor: Binary,
    pub data: Binary,
    pub rewards: Vec<Reward>,
}

#[cw_serde]
pub struct GetServiceContracts {
    pub service_contracts_msg: ServiceMsg,
}

#[cw_serde]
pub struct GetServiceFees {
    pub service_fee_msg: ServiceMsg,
}

#[cw_serde]
pub struct GetBoundExecutorFee {
    pub get_bound_executor_fee: BoundExecutorFeeMsg,
}

#[cw_serde]
pub struct BoundExecutorFeeMsg {}

#[cw_serde]
pub struct GetParticipantFee {
    pub get_participant_fee: ServiceFeesMsg,
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
    pub rewards: Vec<Reward>,
}

#[cw_serde]
pub struct LatestStageResponse {
    pub latest_stage: u64,
}

#[cw_serde]
pub struct CurrentStageResponse {
    pub current_stage: u64,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub new_owner: Option<Addr>,
    pub new_service_addr: Option<Addr>,
    pub new_contract_fee: Option<Coin>,
    pub new_executors: Option<Vec<Binary>>,
    pub old_executors: Option<Vec<Binary>>,
    pub new_checkpoint: Option<u64>,
    pub new_checkpoint_threshold: Option<u64>,
    pub new_max_req_threshold: Option<u64>,
    pub new_trust_period: Option<u64>,
    pub new_slashing_amount: Option<u64>,
    pub new_denom: Option<String>,
    pub new_pending_period: Option<u64>,
}
