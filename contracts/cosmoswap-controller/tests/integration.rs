use cosmoswap_controller::msg::{ExecuteMsg, InstantiateMsg};
use cosmoswap_controller::ContractError;
use cosmoswap_packages::types::SwapInfo;
use cosmwasm_std::coin;
use cosmwasm_std::Decimal;
use cosmwasm_std::{Addr, Empty};
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
            .init_balance(
                storage,
                &Addr::unchecked(USER1),
                vec![coin(1_000_000, DENOM1)],
            )
            .unwrap();
    })
}

fn cosmoswap_controller() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cosmoswap_controller::contract::execute,
        cosmoswap_controller::contract::instantiate,
        cosmoswap_controller::contract::query,
    );
    Box::new(contract)
}

fn cosmoswap() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cosmoswap::contract::execute,
        cosmoswap::contract::instantiate,
        cosmoswap::contract::query,
    );
    Box::new(contract)
}

fn proper_instantiate(app: &mut App, fee_percentage: &str) -> Addr {
    let cosmoswap_controller_code_id = app.store_code(cosmoswap_controller());
    let msg = InstantiateMsg {
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

mod execute {
    use super::*;

    mod create_swap {
        use super::*;

        #[test]
        fn test_happy_path() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, "0.05");

            // Update cosmoswap code id
            let cosmoswap_code_id = app.store_code(cosmoswap());
            let msg = ExecuteMsg::UpdateConfig { cosmoswap_code_id };
            app.execute_contract(
                Addr::unchecked(ADMIN),
                cosmoswap_controller_addr.clone(),
                &msg,
                &vec![],
            )
            .unwrap();

            let swap_info = SwapInfo {
                user1: USER1.to_string(),
                user2: USER2.to_string(),
                coin1: coin(1_000, DENOM1),
                coin2: coin(5_000, DENOM2),
            };
            let msg = ExecuteMsg::CreateSwap { swap_info };
            app.execute_contract(
                Addr::unchecked(USER1),
                cosmoswap_controller_addr.clone(),
                &msg,
                &vec![coin(1_000, DENOM1)],
            )
            .unwrap();

            let res = app.wrap().query_wasm_contract_info("contract1").unwrap();
            assert_eq!(res.code_id, 2);
            assert_eq!(res.creator, cosmoswap_controller_addr);
            assert_eq!(res.admin, None);
        }

        #[test]
        fn test_invalid_user() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, "0.05");

            let swap_info = SwapInfo {
                user1: ADMIN.to_string(),
                user2: USER2.to_string(),
                coin1: coin(1_000, DENOM1),
                coin2: coin(5_000, DENOM2),
            };
            let msg = ExecuteMsg::CreateSwap { swap_info };

            let err = app
                .execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![coin(1_000, DENOM1)],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::Unauthorized {}.to_string()
            );
        }
    }
}
