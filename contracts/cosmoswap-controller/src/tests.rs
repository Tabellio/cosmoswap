use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::Config;
use crate::ContractError;
use cosmoswap_packages::types::FeeInfo;
use cosmwasm_std::Decimal;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use std::str::FromStr;

const ADMIN: &str = "juno..admin";
const USER1: &str = "juno..user1";
const USER2: &str = "juno..user2";

fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(ADMIN), vec![])
            .unwrap();
    })
}

fn cosmoswap_controller() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn proper_instantiate(app: &mut App, cosmoswap_code_id: u64, fee_percentage: &str) -> Addr {
    let code_id = app.store_code(cosmoswap_controller());
    let msg = InstantiateMsg {
        cosmoswap_code_id,
        fee_percentage: Decimal::from_str(fee_percentage).unwrap(),
        fee_payment_address: Addr::unchecked(ADMIN).to_string(),
    };
    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &vec![],
        "cosmoswap-controller",
        None,
    )
    .unwrap()
}

mod instantiate {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut app = mock_app();

        let code_id = app.store_code(cosmoswap_controller());
        let msg = InstantiateMsg {
            cosmoswap_code_id: 2,
            fee_percentage: Decimal::from_str("0.05").unwrap(),
            fee_payment_address: Addr::unchecked(ADMIN).to_string(),
        };

        let cosmoswap_controller_addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked(ADMIN),
                &msg,
                &vec![],
                "cosmoswap-controller",
                None,
            )
            .unwrap();

        assert_eq!(cosmoswap_controller_addr, "contract0");
        let res = app
            .wrap()
            .query_wasm_contract_info(cosmoswap_controller_addr)
            .unwrap();
        assert_eq!(res.code_id, code_id);
        assert_eq!(res.creator, ADMIN);
    }
}

mod execute {
    use super::*;

    mod update_config {
        use super::*;

        #[test]
        fn test_happy_path() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

            let msg = ExecuteMsg::UpdateConfig {
                cosmoswap_code_id: 2,
            };
            app.execute_contract(
                Addr::unchecked(ADMIN),
                cosmoswap_controller_addr.clone(),
                &msg,
                &vec![],
            )
            .unwrap();

            let msg = QueryMsg::Config {};
            let res: Config = app
                .wrap()
                .query_wasm_smart(cosmoswap_controller_addr, &msg)
                .unwrap();
            assert_eq!(res.cosmoswap_code_id, 2);
        }

        #[test]
        fn test_invalid_admin() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

            let msg = ExecuteMsg::UpdateConfig {
                cosmoswap_code_id: 2,
            };
            let err = app
                .execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::Unauthorized {}.to_string()
            );
        }
    }

    mod update_fee_config {
        use super::*;

        #[test]
        fn test_happy_path() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, 2, "0.05");

            let msg = ExecuteMsg::UpdateFeeConfig {
                fee_percentage: Decimal::from_str("0.1").unwrap(),
                fee_payment_address: USER2.to_string(),
            };
            app.execute_contract(
                Addr::unchecked(ADMIN),
                cosmoswap_controller_addr.clone(),
                &msg,
                &vec![],
            )
            .unwrap();

            let msg = QueryMsg::FeeConfig {};
            let res: FeeInfo = app
                .wrap()
                .query_wasm_smart(cosmoswap_controller_addr, &msg)
                .unwrap();
            assert_eq!(res.percentage, Decimal::from_str("0.1").unwrap());
            assert_eq!(res.payment_address, USER2);
        }

        #[test]
        fn test_invalid_admin() {
            let mut app = mock_app();
            let cosmoswap_controller_addr = proper_instantiate(&mut app, 2, "0.05");

            let msg = ExecuteMsg::UpdateFeeConfig {
                fee_percentage: Decimal::from_str("0.1").unwrap(),
                fee_payment_address: USER2.to_string(),
            };
            let err = app
                .execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::Unauthorized {}.to_string()
            );
        }
    }
}
