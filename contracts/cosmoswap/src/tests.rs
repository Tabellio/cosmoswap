use cosmoswap_packages::types::SwapCoin;
use cosmoswap_packages::types::{FeeInfo, SwapInfo};
use cosmwasm_std::Uint128;
use cosmwasm_std::{coin, Addr, Decimal, Empty};
use cw20::Expiration;
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use std::str::FromStr;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::Swap;
use crate::ContractError;

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
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(USER2),
                vec![coin(1_000_000, DENOM2)],
            )
            .unwrap();
    })
}

fn cosmoswap() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn proper_instantiate(app: &mut App, fee_info: FeeInfo, swap_info: SwapInfo) -> Addr {
    let code_id = app.store_code(cosmoswap());
    let msg = InstantiateMsg {
        fee_info,
        swap_info: swap_info.clone(),
        expiration: Expiration::Never {},
    };
    app.instantiate_contract(
        code_id,
        Addr::unchecked(USER1),
        &msg,
        &vec![swap_info.coin1.coin],
        "cosmoswap",
        None,
    )
    .unwrap()
}

mod instantiate {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut app = mock_app();
        let cosmoswap_code_id = app.store_code(cosmoswap());

        let msg = InstantiateMsg {
            fee_info: FeeInfo {
                percentage: Decimal::from_str("0.05").unwrap(),
                payment_address: Addr::unchecked(ADMIN),
            },
            swap_info: SwapInfo {
                user1: Addr::unchecked(USER1).to_string(),
                user2: Addr::unchecked(USER2).to_string(),
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
            },
            expiration: Expiration::Never {},
        };
        let cosmoswap_addr = app
            .instantiate_contract(
                cosmoswap_code_id,
                Addr::unchecked(USER1),
                &msg,
                &vec![coin(1_000, DENOM1)],
                "cosmoswap",
                None,
            )
            .unwrap();
        assert_eq!(cosmoswap_addr, "contract0");
    }
}

mod execute {
    use super::*;

    mod accept {
        use super::*;

        #[test]
        fn test_happy_path() {
            let mut app = mock_app();
            let cosmoswap_addr = proper_instantiate(
                &mut app,
                FeeInfo {
                    percentage: Decimal::from_str("0.05").unwrap(),
                    payment_address: Addr::unchecked(ADMIN),
                },
                SwapInfo {
                    user1: Addr::unchecked(USER1).to_string(),
                    user2: Addr::unchecked(USER2).to_string(),
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
                },
            );

            let msg = ExecuteMsg::Accept {};
            app.execute_contract(
                Addr::unchecked(USER2),
                cosmoswap_addr.clone(),
                &msg,
                &vec![coin(5_000, DENOM2)],
            )
            .unwrap();

            let swap: Swap = app
                .wrap()
                .query_wasm_smart(cosmoswap_addr, &QueryMsg::Swap {})
                .unwrap();
            assert_eq!(swap.user1, Addr::unchecked(USER1));
            assert_eq!(swap.user2, Addr::unchecked(USER2));
            assert_eq!(swap.coin1.coin, coin(1_000, DENOM1));
            assert_eq!(swap.coin2.coin, coin(5_000, DENOM2));
        }

        #[test]
        fn test_locked_swap() {
            let mut app = mock_app();
            let cosmoswap_addr = proper_instantiate(
                &mut app,
                FeeInfo {
                    percentage: Decimal::from_str("0.05").unwrap(),
                    payment_address: Addr::unchecked(ADMIN),
                },
                SwapInfo {
                    user1: Addr::unchecked(USER1).to_string(),
                    user2: Addr::unchecked(USER2).to_string(),
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
                },
            );

            let msg = ExecuteMsg::Cancel {};
            app.execute_contract(
                Addr::unchecked(USER1),
                cosmoswap_addr.clone(),
                &msg,
                &vec![],
            )
            .unwrap();

            let msg = ExecuteMsg::Accept {};
            let err = app
                .execute_contract(
                    Addr::unchecked(USER2),
                    cosmoswap_addr.clone(),
                    &msg,
                    &vec![coin(5_000, DENOM2)],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::SwapLocked {}.to_string()
            );
        }

        #[test]
        fn test_invalid_user() {
            let mut app = mock_app();
            let cosmoswap_addr = proper_instantiate(
                &mut app,
                FeeInfo {
                    percentage: Decimal::from_str("0.05").unwrap(),
                    payment_address: Addr::unchecked(ADMIN),
                },
                SwapInfo {
                    user1: Addr::unchecked(USER1).to_string(),
                    user2: Addr::unchecked(ADMIN).to_string(),
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
                },
            );

            let msg = ExecuteMsg::Accept {};
            let err = app
                .execute_contract(
                    Addr::unchecked(USER2),
                    cosmoswap_addr.clone(),
                    &msg,
                    &vec![coin(5_000, DENOM2)],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::Unauthorized {}.to_string()
            );
        }
    }

    mod cancel {
        use super::*;

        #[test]
        fn test_happy_path() {
            let mut app = mock_app();
            let cosmoswap_addr = proper_instantiate(
                &mut app,
                FeeInfo {
                    percentage: Decimal::from_str("0.05").unwrap(),
                    payment_address: Addr::unchecked(ADMIN),
                },
                SwapInfo {
                    user1: Addr::unchecked(USER1).to_string(),
                    user2: Addr::unchecked(USER2).to_string(),
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
                },
            );

            let res = app.wrap().query_balance(USER1, DENOM1).unwrap();
            assert_eq!(res.amount, Uint128::new(999_000));

            let msg = ExecuteMsg::Cancel {};
            app.execute_contract(
                Addr::unchecked(USER1),
                cosmoswap_addr.clone(),
                &msg,
                &vec![],
            )
            .unwrap();

            let res = app.wrap().query_balance(USER1, DENOM1).unwrap();
            assert_eq!(res.amount, Uint128::new(1_000_000));

            let msg = ExecuteMsg::Accept {};
            let err = app
                .execute_contract(
                    Addr::unchecked(USER2),
                    cosmoswap_addr.clone(),
                    &msg,
                    &vec![],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::SwapLocked {}.to_string()
            )
        }

        #[test]
        fn test_invalid_user() {
            let mut app = mock_app();
            let cosmoswap_addr = proper_instantiate(
                &mut app,
                FeeInfo {
                    percentage: Decimal::from_str("0.05").unwrap(),
                    payment_address: Addr::unchecked(ADMIN),
                },
                SwapInfo {
                    user1: Addr::unchecked(USER1).to_string(),
                    user2: Addr::unchecked(USER2).to_string(),
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
                },
            );

            let msg = ExecuteMsg::Cancel {};
            let err = app
                .execute_contract(
                    Addr::unchecked(USER2),
                    cosmoswap_addr.clone(),
                    &msg,
                    &vec![],
                )
                .unwrap_err();
            assert_eq!(
                err.source().unwrap().to_string(),
                ContractError::Unauthorized {}.to_string()
            )
        }
    }
}
