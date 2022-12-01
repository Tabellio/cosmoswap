use cosmoswap_controller::msg::ReceiveMsg;
use cosmoswap_controller::msg::{ExecuteMsg, InstantiateMsg};
use cosmoswap_controller::ContractError;
use cosmoswap_packages::funds::FundsError;
use cosmoswap_packages::types::SwapCoin;
use cosmoswap_packages::types::SwapInfo;
use cosmwasm_std::coin;
use cosmwasm_std::Decimal;
use cosmwasm_std::{to_binary, Uint128};
use cosmwasm_std::{Addr, Empty};
use cw20::Cw20Coin;
use cw20::Expiration;
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
    )
    .with_reply(cosmoswap_controller::contract::reply);
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
    let msg = InstantiateMsg {
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
            address: USER1.to_string(),
            amount: Uint128::new(1_000_000),
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

mod execute {
    use super::*;

    mod create_swap {
        use super::*;

        mod native_token {
            use super::*;

            #[test]
            fn test_happy_path() {
                let mut app = mock_app();
                let cosmoswap_code_id = app.store_code(cosmoswap());
                let cosmoswap_controller_addr =
                    proper_instantiate(&mut app, cosmoswap_code_id, "0.05");

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
                let msg = ExecuteMsg::CreateSwap {
                    swap_info: swap_info.clone(),
                    expiration: Expiration::Never {},
                };
                app.execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![coin(1_000, DENOM1)],
                )
                .unwrap();

                let res = app.wrap().query_wasm_contract_info("contract1").unwrap();
                assert_eq!(res.code_id, 1);
                assert_eq!(res.creator, cosmoswap_controller_addr);
                assert_eq!(res.admin, None);

                // Creating swap with expiration
                // Changing block time
                // app.update_block(|block| block.time = block.time.plus_seconds(10));
                let msg = ExecuteMsg::CreateSwap {
                    swap_info: swap_info.clone(),
                    expiration: Expiration::AtTime(app.block_info().time.plus_seconds(5)),
                };
                app.execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![coin(1_000, DENOM1)],
                )
                .unwrap();

                // Creating swap with expiration
                // Changing block height
                // app.update_block(|block| block.height = block.height.checked_add(10).unwrap());
                let msg = ExecuteMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::AtHeight(
                        app.block_info().height.checked_add(5).unwrap(),
                    ),
                };
                app.execute_contract(
                    Addr::unchecked(USER1),
                    cosmoswap_controller_addr.clone(),
                    &msg,
                    &vec![coin(1_000, DENOM1)],
                )
                .unwrap();
            }

            #[test]
            fn test_invalid_user() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

                let swap_info = SwapInfo {
                    user1: ADMIN.to_string(),
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
                let msg = ExecuteMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };

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

            #[test]
            fn test_invalid_expiration() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

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

                app.update_block(|block| block.height = block.height.checked_add(10).unwrap());
                let msg = ExecuteMsg::CreateSwap {
                    swap_info: swap_info.clone(),
                    expiration: Expiration::AtHeight(1),
                };
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
                    ContractError::InvalidExpiration {}.to_string()
                );

                app.update_block(|block| block.time = block.time.plus_seconds(10));
                let msg = ExecuteMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::AtTime(app.block_info().time.minus_seconds(9)),
                };
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
                    ContractError::InvalidExpiration {}.to_string()
                );
            }
        }

        mod cw20_token {
            use cw20::{BalanceResponse, Cw20QueryMsg};

            use super::*;

            #[test]
            fn test_happy_path() {
                let mut app = mock_app();
                let cosmoswap_code_id = app.store_code(cosmoswap());
                let cosmoswap_controller_addr =
                    proper_instantiate(&mut app, cosmoswap_code_id, "0.05");

                let cw20_addr = setup_cw20_token(&mut app);

                let swap_info = SwapInfo {
                    user1: USER1.to_string(),
                    user2: USER2.to_string(),
                    coin1: SwapCoin {
                        is_native: false,
                        coin: coin(1_000, CW20_TICKER),
                        cw20_address: Some(cw20_addr.to_string()),
                    },
                    coin2: SwapCoin {
                        is_native: true,
                        coin: coin(5_000, DENOM2),
                        cw20_address: None,
                    },
                };
                let msg = ReceiveMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };
                app.execute_contract(
                    Addr::unchecked(USER1),
                    cw20_addr.clone(),
                    &Cw20ExecuteMsg::Send {
                        contract: cosmoswap_controller_addr.to_string(),
                        amount: Uint128::new(1_000),
                        msg: to_binary(&msg).unwrap(),
                    },
                    &vec![],
                )
                .unwrap();

                let res = app.wrap().query_wasm_contract_info("contract2").unwrap();
                assert_eq!(res.code_id, 1);
                assert_eq!(res.creator, cosmoswap_controller_addr);
                assert_eq!(res.admin, None);

                let res: BalanceResponse = app
                    .wrap()
                    .query_wasm_smart(
                        cw20_addr,
                        &Cw20QueryMsg::Balance {
                            address: "contract2".to_string(),
                        },
                    )
                    .unwrap();
                assert_eq!(res.balance, Uint128::new(1_000));
            }

            #[test]
            fn test_invalid_user() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

                let cw20_addr = setup_cw20_token(&mut app);

                let swap_info = SwapInfo {
                    user1: ADMIN.to_string(),
                    user2: USER2.to_string(),
                    coin1: SwapCoin {
                        is_native: false,
                        coin: coin(1_000, CW20_TICKER),
                        cw20_address: Some(cw20_addr.to_string()),
                    },
                    coin2: SwapCoin {
                        is_native: true,
                        coin: coin(5_000, DENOM2),
                        cw20_address: None,
                    },
                };
                let msg = ReceiveMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };

                let err = app
                    .execute_contract(
                        Addr::unchecked(USER1),
                        cw20_addr.clone(),
                        &Cw20ExecuteMsg::Send {
                            contract: cosmoswap_controller_addr.to_string(),
                            amount: Uint128::new(1_000),
                            msg: to_binary(&msg).unwrap(),
                        },
                        &vec![],
                    )
                    .unwrap_err();
                assert_eq!(
                    err.source().unwrap().source().unwrap().to_string(),
                    ContractError::Unauthorized {}.to_string()
                );
            }

            #[test]
            fn test_invalid_amount() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

                let cw20_addr = setup_cw20_token(&mut app);

                let swap_info = SwapInfo {
                    user1: USER1.to_string(),
                    user2: USER2.to_string(),
                    coin1: SwapCoin {
                        is_native: false,
                        coin: coin(2_000, CW20_TICKER),
                        cw20_address: Some(cw20_addr.to_string()),
                    },
                    coin2: SwapCoin {
                        is_native: true,
                        coin: coin(5_000, DENOM2),
                        cw20_address: None,
                    },
                };
                let msg = ReceiveMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };

                let err = app
                    .execute_contract(
                        Addr::unchecked(USER1),
                        cw20_addr.clone(),
                        &Cw20ExecuteMsg::Send {
                            contract: cosmoswap_controller_addr.to_string(),
                            amount: Uint128::new(1_000),
                            msg: to_binary(&msg).unwrap(),
                        },
                        &vec![],
                    )
                    .unwrap_err();
                assert_eq!(
                    err.source().unwrap().source().unwrap().to_string(),
                    FundsError::InvalidFunds {
                        got: "1000".to_string(),
                        expected: "2000".to_string()
                    }
                    .to_string()
                );
            }

            #[test]
            fn test_invalid_denom() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

                let cw20_addr = setup_cw20_token(&mut app);

                let swap_info = SwapInfo {
                    user1: USER1.to_string(),
                    user2: USER2.to_string(),
                    coin1: SwapCoin {
                        is_native: false,
                        coin: coin(1_000, "invalid"),
                        cw20_address: Some(cw20_addr.to_string()),
                    },
                    coin2: SwapCoin {
                        is_native: true,
                        coin: coin(5_000, DENOM2),
                        cw20_address: None,
                    },
                };
                let msg = ReceiveMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };

                let err = app
                    .execute_contract(
                        Addr::unchecked(USER1),
                        cw20_addr.clone(),
                        &Cw20ExecuteMsg::Send {
                            contract: cosmoswap_controller_addr.to_string(),
                            amount: Uint128::new(1_000),
                            msg: to_binary(&msg).unwrap(),
                        },
                        &vec![],
                    )
                    .unwrap_err();
                assert_eq!(
                    err.source().unwrap().source().unwrap().to_string(),
                    FundsError::InvalidDenom {
                        got: "invalid".to_string(),
                        expected: CW20_TICKER.to_string()
                    }
                    .to_string()
                );
            }

            #[test]
            fn test_invalid_cw20_address() {
                let mut app = mock_app();
                let cosmoswap_controller_addr = proper_instantiate(&mut app, 1, "0.05");

                let cw20_addr = setup_cw20_token(&mut app);

                let swap_info = SwapInfo {
                    user1: USER1.to_string(),
                    user2: USER2.to_string(),
                    coin1: SwapCoin {
                        is_native: false,
                        coin: coin(1_000, CW20_TICKER),
                        cw20_address: None,
                    },
                    coin2: SwapCoin {
                        is_native: true,
                        coin: coin(5_000, DENOM2),
                        cw20_address: None,
                    },
                };
                let msg = ReceiveMsg::CreateSwap {
                    swap_info,
                    expiration: Expiration::Never {},
                };

                let err = app
                    .execute_contract(
                        Addr::unchecked(USER1),
                        cw20_addr.clone(),
                        &Cw20ExecuteMsg::Send {
                            contract: cosmoswap_controller_addr.to_string(),
                            amount: Uint128::new(1_000),
                            msg: to_binary(&msg).unwrap(),
                        },
                        &vec![],
                    )
                    .unwrap_err();
                assert_eq!(
                    err.source().unwrap().source().unwrap().to_string(),
                    ContractError::InvalidCw20Addr {}.to_string()
                );
            }
        }
    }
}
