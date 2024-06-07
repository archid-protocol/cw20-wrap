use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use cw20_base::contract::{execute_burn, execute_mint};
use cw20_base::state::BALANCES;

use crate::error::ContractError;
use crate::state::STATE;

pub fn try_deposit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    let deposit = info
        .funds
        .iter()
        .find(|x| x.denom == state.native_coin)
        .ok_or(ContractError::InvalidDeposit {
            denom: state.native_coin.clone(),
        })?;

    if u128::from(deposit.amount) == 0_u128 {
        return Err(ContractError::InvalidDeposit {
            denom: state.native_coin,
        });
    }

    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    execute_mint(
        deps,
        env,
        sub_info,
        info.sender.clone().into(),
        deposit.amount,
    )?;

    let res = Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("amount", deposit.amount)
        .add_attribute("sender", info.sender);
    Ok(res)
}

pub fn try_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Ensure user has balance sufficient to cover withdrawal
    let balance = BALANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    if u128::from(amount) > u128::from(balance) {
        return Err(ContractError::InvalidWithdrawal {
            withdrawal: amount.into(),
            balance: balance.into(),
        });
    }

    // Burn coins
    execute_burn(deps, env, info.clone(), amount)?;

    // Return native coin
    let bank_send = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.clone().into(),
        amount: vec![Coin::new(amount.into(), state.native_coin)],
    });

    let res = Response::new()
        .add_attribute("action", "withdraw")
        .add_attribute("amount", amount)
        .add_attribute("sender", info.sender)
        .add_message(bank_send);
    Ok(res)
}
