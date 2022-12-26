use cosmoswap::msg::ExecuteMsg;
use cosmoswap::msg::ReceiveMsg;
use cosmoswap::ContractError;
use cosmoswap_controller::msg::{
    ExecuteMsg as ControllerExecuteMsg, InstantiateMsg as ControllerInstantiateMsg,
};
use cosmoswap_packages::types::{SwapCoin, SwapInfo};
use cosmwasm_std::to_binary;
use cosmwasm_std::{coin, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw20::Expiration;
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw20_base::msg::{ExecuteMsg as Cw20ExecuteMsg, InstantiateMsg as Cw20InstantiateMsg};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use std::str::FromStr;

const ADMIN: &str = "juno..admin";
const USER1: &str = "juno..user1";
const USER2: &str = "juno..user2";
const DENOM1: &str = "denom1";
const DENOM2: &str = "denom2";
const CW20_TICKER: &str = "teto";

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
    )
    .with_reply(cosmoswap_controller::contract::reply);
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn proper_instantiate(app: &mut App, cosmoswap_code_id: u64, fee_percentage: &str) -> Addr {
    let cosmoswap_controller_code_id = app.store_code(cosmoswap_controller());
    let msg = ControllerInstantiateMsg {
        cosmoswap_code_id,
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

fn setup_cw20_token(app: &mut App) -> Addr {
    let cw20_code_id = app.store_code(cw20_contract());

    // Create a new cw20 token
    let msg = Cw20InstantiateMsg {
        name: "Test Token".to_string(),
        symbol: CW20_TICKER.to_string(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: USER2.to_string(),
            amount: Uint128::new(10_000),
        }],
        marketing: None,
        mint: None,
    };
    app.instantiate_contract(
        cw20_code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &vec![],
        "test cw20",
        None,
    )
    .unwrap()
}

mod native_token {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut app = mock_app();
        let cosmoswap_code_id = app.store_code(cosmoswap());
        let cosmoswap_controller_addr = proper_instantiate(&mut app, cosmoswap_code_id, "0.05");

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
                expiration: Expiration::Never {},
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
                expiration: Expiration::Never {},
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

        let msg = ExecuteMsg::Accept {};
        let err = app
            .execute_contract(
                Addr::unchecked(USER2),
                Addr::unchecked("contract1"),
                &msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            err.source().unwrap().to_string(),
            ContractError::SwapLocked {}.to_string()
        );

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
}

mod cw20_token {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut app = mock_app();
        let cosmoswap_code_id = app.store_code(cosmoswap());
        let cosmoswap_controller_addr = proper_instantiate(&mut app, cosmoswap_code_id, "0.05");

        let cw20_addr = setup_cw20_token(&mut app);

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
                is_native: false,
                coin: coin(5_000, CW20_TICKER),
                cw20_address: Some(cw20_addr.to_string()),
            },
        };
        // Contract2
        app.execute_contract(
            Addr::unchecked(USER1),
            cosmoswap_controller_addr.clone(),
            &ControllerExecuteMsg::CreateSwap {
                swap_info: swap_info.clone(),
                expiration: Expiration::Never {},
            },
            &vec![swap_info.clone().coin1.coin],
        )
        .unwrap();
        // Contract3
        app.execute_contract(
            Addr::unchecked(USER1),
            cosmoswap_controller_addr.clone(),
            &ControllerExecuteMsg::CreateSwap {
                swap_info: swap_info.clone(),
                expiration: Expiration::Never {},
            },
            &vec![swap_info.clone().coin1.coin],
        )
        .unwrap();

        // cosmoswap address is contract1
        app.execute_contract(
            Addr::unchecked(USER2),
            cw20_addr.clone(),
            &Cw20ExecuteMsg::Send {
                contract: "contract2".to_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&ReceiveMsg::Accept {}).unwrap(),
            },
            &vec![],
        )
        .unwrap();

        // Initial balances are zero
        let res = app.wrap().query_balance(USER1, DENOM1).unwrap();
        assert_eq!(res.amount, Uint128::zero());
        let res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &Cw20QueryMsg::Balance {
                    address: USER2.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(5_000));

        // These are the balances after the swap
        let res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &Cw20QueryMsg::Balance {
                    address: USER1.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(4750));
        let res = app.wrap().query_balance(USER2, DENOM1).unwrap();
        assert_eq!(res.amount, Uint128::new(950));

        // Admin fee
        let res = app.wrap().query_balance(ADMIN, DENOM1).unwrap();
        assert_eq!(res.amount, Uint128::new(50));
        let res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &Cw20QueryMsg::Balance {
                    address: ADMIN.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(250));

        // Cancel the second swap
        let msg = ExecuteMsg::Cancel {};
        app.execute_contract(
            Addr::unchecked(USER1),
            Addr::unchecked("contract3"),
            &msg,
            &vec![],
        )
        .unwrap();

        let err = app
            .execute_contract(
                Addr::unchecked(USER2),
                cw20_addr.clone(),
                &Cw20ExecuteMsg::Send {
                    contract: "contract3".to_string(),
                    amount: Uint128::new(5_000),
                    msg: to_binary(&ReceiveMsg::Accept {}).unwrap(),
                },
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            err.source().unwrap().source().unwrap().to_string(),
            ContractError::SwapLocked {}.to_string()
        )
    }
}
