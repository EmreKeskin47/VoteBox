use std::os::macos::raw::stat;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;
use cw_utils::Scheduled;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, VoteResponse};
use crate::msg::ExecuteMsg::vote_reset;
use crate::state::{State, STATE, Vote};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:vote";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if "admin" != info.sender {
        Err(ContractError::Unauthorized {})
    } else {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        let state = State {
            vote: Vote { yes_count: Uint128::zero(), no_count: Uint128::zero() },
            deadline: msg.deadline,
        };
        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_attribute("method", "instantiate").add_attribute("yes_count", "0").add_attribute("no_count", "0"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::vote_yes => vote_yes(deps, env),
        ExecuteMsg::vote_no => vote_no(deps, env),
        ExecuteMsg::vote_reset => reset(deps, env, info),
    }
}

pub fn vote_yes(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut param: Uint128 = Uint128::zero();
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.deadline.is_triggered(&env.block) {
            return Err(ContractError::Expired {});
        }
        state.vote.yes_count += Uint128::new(1);
        param = state.vote.yes_count;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "vote_yes").add_attribute("yes_count", param))
}

pub fn vote_no(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut param: Uint128 = Uint128::zero();
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.deadline.is_triggered(&env.block) {
            return Err(ContractError::Expired {});
        }
        state.vote.no_count += Uint128::new(1);
        param = state.vote.no_count;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "vote_no").add_attribute("no_count", param))
}

pub fn reset(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut param1: Uint128 = Uint128::zero();
    let mut param2: Uint128 = Uint128::zero();
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.deadline.is_triggered(&env.block) {
            return Err(ContractError::Expired {});
        } else if "admin" != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        state.vote.yes_count = Uint128::zero();
        state.vote.no_count = Uint128::zero();
        param1 = state.vote.yes_count;
        param2 = state.vote.no_count;

        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("method", "vote_reset")
        .add_attribute("yes_count", param1)
        .add_attribute("no_count", param2))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<VoteResponse> {
    match msg {
        QueryMsg::query_vote => query_vote(deps),
    }
}

pub fn query_vote(deps: Deps) -> StdResult<VoteResponse> {
    let vote_item = STATE.load(deps.storage)?;
    Ok(VoteResponse {
        vote: vote_item
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use cw_utils::Scheduled;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let msg = InstantiateMsg { deadline: Scheduled::AtHeight(123) };
        let info = mock_info("admin", &coins(1000, "earth"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let value = res.attributes;
        assert_eq!("0", value[1].value);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let intmsg = InstantiateMsg { deadline: Scheduled::AtHeight(123111) };
        let msg = ExecuteMsg::vote_yes;
        let intinfo = mock_info("admin", &coins(1000, "earth"));
        let info = mock_info("admin", &coins(1000, "earth"));
        let intres = instantiate(deps.as_mut(), mock_env(), intinfo, intmsg).unwrap();
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let value = res.attributes;
        assert_eq!("2", value[1].value, "initial value is {}", value[1].value);
    }

    #[test]
    fn decrement() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let intmsg = InstantiateMsg { deadline: Scheduled::AtHeight(123111) };
        let msg = ExecuteMsg::vote_no;
        let intinfo = mock_info("admin", &coins(1000, "earth"));
        let info = mock_info("admin", &coins(1000, "earth"));
        let intres = instantiate(deps.as_mut(), mock_env(), intinfo, intmsg).unwrap();
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let value = res.attributes;
        assert_eq!("2", value[1].value);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let intmsg = InstantiateMsg { deadline: Scheduled::AtHeight(123111) };
        let msg = ExecuteMsg::vote_reset;
        let intinfo = mock_info("admin", &coins(1000, "earth"));
        let info = mock_info("admin", &coins(1000, "earth"));
        let intres = instantiate(deps.as_mut(), mock_env(), intinfo, intmsg).unwrap();
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let value = res.attributes;
        assert_eq!("0", value[1].value);
        assert_eq!("0", value[2].value);
    }

    #[test]
    fn query_test() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let intmsg = InstantiateMsg { deadline: Scheduled::AtHeight(123111) };
        let msg = QueryMsg::query_vote;
        let intinfo = mock_info("admin", &coins(1000, "earth"));
        let info = mock_info("admin", &coins(1000, "earth"));
        let intres = instantiate(deps.as_mut(), mock_env(), intinfo, intmsg).unwrap();
        let res = query(deps.as_ref(), mock_env(), msg.clone()).unwrap();
    }
}
