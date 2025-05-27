#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};

use cw2::set_contract_version;
use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_burn, execute_send, execute_transfer, query_balance, query_minter, query_token_info,
};
use cw20_base::enumerable::{query_all_accounts, query_owner_allowances};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use crate::error::ContractError;
use crate::execute::{try_deposit, try_withdraw};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// Contract metadata
const CONTRACT_NAME: &str = "crates.io:cw20-wrap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    let state = State {
        owner: info.sender,
        native_coin: msg.native_coin,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => try_deposit(deps, env, info),
        ExecuteMsg::Withdraw { amount } => try_withdraw(deps, env, info, amount),

        // cw20 standard
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(execute_transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Burn { amount } => Ok(execute_burn(deps, env, info, amount)?),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(execute_send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(execute_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        ExecuteMsg::BurnFrom { owner, amount } => {
            Ok(execute_burn_from(deps, env, info, owner, amount)?)
        }
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(execute_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // cw20 standard
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_json_binary(&query_all_accounts(deps, start_after, limit)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coin, coins, from_json, BankMsg, CosmosMsg, SubMsg};
    use cw20::BalanceResponse;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            native_coin: "aarch".into(),
            name: "wrapped arch".into(),
            decimals: 18.into(),
            symbol: "WARCH".into(),
        };
        let info = mock_info("creator", &[]);

        // Calling .unwrap() will assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn deposit() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            native_coin: "aarch".into(),
            name: "wrapped arch".into(),
            decimals: 18.into(),
            symbol: "WARCH".into(),
        };
        let info = mock_info("creator", &[]);

        // Calling .unwrap() will assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Depositing invalid coin fails
        let env = mock_env();
        let info = mock_info("anyone", &coins(10, "btc"));
        let err = try_deposit(deps.as_mut(), env.clone(), info);
        assert!(err.is_err());

        // Deposits of 0 aarch fail
        let info = mock_info("creator", &coins(0_u128, "aarch")); // 0 ARCH
        let err = try_deposit(deps.as_mut(), env.clone(), info);
        assert!(err.is_err());

        // Depositing a positive amount of aarch succeeds
        let info = mock_info("creator", &coins(10000000000000000000_u128, "aarch")); // 10 ARCH
        let res = try_deposit(deps.as_mut(), env.clone(), info).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Verify balance was updated
        let data = query(
            deps.as_ref(),
            env,
            QueryMsg::Balance {
                address: String::from("creator"),
            },
        )
        .unwrap();
        let response: BalanceResponse = from_json(&data).unwrap();
        assert_eq!(response.balance, Uint128::from(10000000000000000000_u128));
    }

    #[test]
    fn withdraw() {
        let mut deps = mock_dependencies_with_balance(&coins(1000u32.into(), "arch_owner"));

        let msg = InstantiateMsg {
            native_coin: "aarch".into(),
            name: "wrapped arch".into(),
            decimals: 18.into(),
            symbol: "WARCH".into(),
        };
        let info = mock_info("creator", &[]);

        // Calling .unwrap() will assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Deposit
        let env = mock_env();
        let amount_deposit = 5000000000000000000_u128; // 5 ARCH
        let info = mock_info("creator", &coins(amount_deposit.into(), "aarch"));
        let res = try_deposit(deps.as_mut(), env.clone(), info).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Random cannot withdraw
        let info = mock_info("random", &[]);
        let amount_withdraw = 5000000000000000000_u128; // 5 ARCH
        let err = try_withdraw(deps.as_mut(), env.clone(), info, amount_withdraw.into());
        assert!(err.is_err());

        // Owner cannot withdraw more funds than available
        let info = mock_info("creator", &[]);
        let amount_withdraw = 10000000000000000000_u128; // 10 ARCH
        let err = try_withdraw(deps.as_mut(), env.clone(), info, amount_withdraw.into());
        assert!(err.is_err());

        // Owner can withdraw less funds than available
        let info = mock_info("creator", &[]);
        let amount_withdraw = 4000000000000000000_u128; // 4 ARCH
        let res = try_withdraw(deps.as_mut(), env.clone(), info, amount_withdraw.into()).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                amount: vec![coin(amount_withdraw.into(), "aarch")],
                to_address: "creator".into(),
            }))
        );
        let data = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Balance {
                address: String::from("creator"),
            },
        )
        .unwrap();
        let balance_response: BalanceResponse = from_json(&data).unwrap();
        assert_eq!(
            balance_response.balance,
            Uint128::from(1000000000000000000_u128) // 1 ARCH remains to be withdrawn
        );

        // Owner can withdraw exact funds available
        let info = mock_info("creator", &[]);
        let amount_withdraw = 1000000000000000000_u128; // 1 ARCH
        let res = try_withdraw(deps.as_mut(), env.clone(), info, amount_withdraw.into()).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                amount: vec![coin(amount_withdraw.into(), "aarch")],
                to_address: "creator".into(),
            }))
        );
        let data = query(
            deps.as_ref(),
            env,
            QueryMsg::Balance {
                address: String::from("creator"),
            },
        )
        .unwrap();
        let balance_response: BalanceResponse = from_json(&data).unwrap();
        assert_eq!(
            balance_response.balance,
            Uint128::from(0_u8) // All ARCH withdrawn
        );
    }
}
