use cosmoswap::msg::ExecuteMsg;
use cosmoswap::ContractError;
use cosmoswap_controller::msg::{
    ExecuteMsg as ControllerExecuteMsg, InstantiateMsg as ControllerInstantiateMsg,
};
use cosmoswap_packages::types::{SwapCoin, SwapInfo};
use cosmwasm_std::{coin, Addr, Decimal, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use std::str::FromStr;

const ADMIN: &str = "juno..admin";
const USER1: &str = "juno..user1";
const USER2: &str = "juno..user2";
const DENOM1: &str = "denom1";
const DENOM2: &str = "denom2";

fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(USER1), vec![coin(2000, DENOM1)])
            .unwrap();
        router
            .bank
            .init_balance(storage, &Addr::unchecked(USER2), vec![coin(5000, DENOM2)])
            .unwrap();
    })
}

fn cosmoswap() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cosmoswap::contract::execute,
        cosmoswap::contract::instantiate,
        cosmoswap::contract::query,
    );
    Box::new(contract)
}

fn cosmoswap_controller() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cosmoswap_controller::contract::execute,
        cosmoswap_controller::contract::instantiate,
        cosmoswap_controller::contract::query,
    );
    Box::new(contract)
}

fn proper_instantiate(app: &mut App, fee_percentage: &str) -> Addr {
    let cosmoswap_controller_code_id = app.store_code(cosmoswap_controller());
    let msg = ControllerInstantiateMsg {
        fee_percentage: Decimal::from_str(fee_percentage).unwrap(),
        fee_payment_address: Addr::unchecked(ADMIN).to_string(),
    };
    app.instantiate_contract(
        cosmoswap_controller_code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &vec![],
        "cosmoswap-controller",
        None,
    )
    .unwrap()
}

fn update_cosmoswap_code_id(app: &mut App, cosmoswap_controller_addr: Addr) {
    let cosmoswap_code_id = app.store_code(cosmoswap());
    app.execute_contract(
        Addr::unchecked(ADMIN),
        cosmoswap_controller_addr,
        &ControllerExecuteMsg::UpdateConfig { cosmoswap_code_id },
        &vec![],
    )
    .unwrap();
}

#[test]
fn test_happy_path() {
    let mut app = mock_app();
    let cosmoswap_controller_addr = proper_instantiate(&mut app, "0.05");

    // Upload cosmoswap code and update controller
    update_cosmoswap_code_id(&mut app, cosmoswap_controller_addr.clone());

    // Create swap
    let swap_info = SwapInfo {
        user1: USER1.to_string(),
        user2: USER2.to_string(),
        coin1: SwapCoin {
            is_native: true,
            coin: coin(1_000, DENOM1),
            cw20_address: None,
        },
        coin2: SwapCoin {
            is_native: true,
            coin: coin(5_000, DENOM2),
            cw20_address: None,
        },
    };
    // Contract1
    app.execute_contract(
        Addr::unchecked(USER1),
        cosmoswap_controller_addr.clone(),
        &ControllerExecuteMsg::CreateSwap {
            swap_info: swap_info.clone(),
        },
        &vec![swap_info.clone().coin1.coin],
    )
    .unwrap();
    // Contract2
    app.execute_contract(
        Addr::unchecked(USER1),
        cosmoswap_controller_addr.clone(),
        &ControllerExecuteMsg::CreateSwap {
            swap_info: swap_info.clone(),
        },
        &vec![swap_info.clone().coin1.coin],
    )
    .unwrap();

    // cosmoswap address is contract1
    let msg = ExecuteMsg::Accept {};
    app.execute_contract(
        Addr::unchecked(USER2),
        Addr::unchecked("contract1"),
        &msg,
        &vec![swap_info.clone().coin2.coin],
    )
    .unwrap();

    // Initial balances are zero
    let res = app.wrap().query_balance(USER1, DENOM1).unwrap();
    assert_eq!(res.amount, Uint128::zero());
    let res = app.wrap().query_balance(USER2, DENOM2).unwrap();
    assert_eq!(res.amount, Uint128::zero());

    // These are the balances after the swap
    let res = app.wrap().query_balance(USER1, DENOM2).unwrap();
    assert_eq!(res.amount, Uint128::new(4750));
    let res = app.wrap().query_balance(USER2, DENOM1).unwrap();
    assert_eq!(res.amount, Uint128::new(950));

    // Admin fee
    let res = app.wrap().query_balance(ADMIN, DENOM1).unwrap();
    assert_eq!(res.amount, Uint128::new(50));
    let res = app.wrap().query_balance(ADMIN, DENOM2).unwrap();
    assert_eq!(res.amount, Uint128::new(250));

    // Cancel the second swap
    let msg = ExecuteMsg::Cancel {};
    app.execute_contract(
        Addr::unchecked(USER1),
        Addr::unchecked("contract2"),
        &msg,
        &vec![],
    )
    .unwrap();

    let msg = ExecuteMsg::Accept {};
    let err = app
        .execute_contract(
            Addr::unchecked(USER2),
            Addr::unchecked("contract2"),
            &msg,
            &vec![],
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::SwapLocked {}.to_string()
    )
}
