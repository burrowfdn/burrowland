use common::ONE_YOCTO;
use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{env, Balance, Gas, Timestamp};
use near_sdk_sim::runtime::GenesisConfig;
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, ExecutionResult, UserAccount,
};
use std::convert::TryInto;

use contract::{AssetConfig, Config, ContractContract as BurrowlandContract};
use test_oracle::ContractContract as OracleContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    BURROWLAND_WASM_BYTES => "res/burrowland.wasm",
    TEST_ORACLE_WASM_BYTES => "res/test_oracle.wasm",

    FUNGIBLE_TOKEN_WASM_BYTES => "res/fungible_token.wasm",
}

pub const NEAR: &str = "near";
pub const ORACLE_ID: &str = "oracle.near";
pub const BURROWLAND_ID: &str = "burrowland.near";
pub const BOOSTER_TOKEN_ID: &str = "token.burrowland.near";
pub const OWNER_ID: &str = "owner.near";

pub const T_GAS: Gas = 1_000_000_000_000;
pub const DEFAULT_GAS: Gas = 15 * T_GAS;
pub const MAX_GAS: Gas = 300 * T_GAS;
pub const BOOSTER_TOKEN_DECIMALS: u8 = 18;
pub const BOOSTER_TOKEN_TOTAL_SUPPLY: Balance =
    1_000_000_000 * 10u128.pow(BOOSTER_TOKEN_DECIMALS as _);

pub const DEPOSIT_TO_RESERVE: &str = "\"DepositToReserve\"";

pub struct Env {
    pub root: UserAccount,
    pub near: UserAccount,
    pub owner: UserAccount,
    pub oracle: ContractAccount<OracleContract>,
    pub contract: ContractAccount<BurrowlandContract>,
    pub booster_token: UserAccount,
}

pub struct Tokens {
    pub wnear: UserAccount,
    pub neth: UserAccount,
    pub ndai: UserAccount,
    pub nusdt: UserAccount,
    pub nusdc: UserAccount,
}

pub struct Users {
    pub alice: UserAccount,
    pub bob: UserAccount,
    pub charlie: UserAccount,
    pub dude: UserAccount,
    pub eve: UserAccount,
}

pub fn storage_deposit(user: &UserAccount, token_account_id: &str, account_id: &str) {
    user.call(
        token_account_id.to_string(),
        "storage_deposit",
        &json!({
            "account_id": account_id.to_string()
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        125 * env::STORAGE_PRICE_PER_BYTE, // attached deposit
    )
    .assert_success();
}

pub fn to_nano(timestamp: u32) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}

impl Env {
    pub fn init() -> Self {
        let mut genesis_config = GenesisConfig::default();
        genesis_config.block_prod_time = 0;
        let root = init_simulator(Some(genesis_config));
        let near = root.create_user(NEAR.to_string(), to_yocto("1000000"));
        let owner = near.create_user(OWNER_ID.to_string(), to_yocto("10000"));

        let oracle = deploy!(
            contract: OracleContract,
            contract_id: ORACLE_ID.to_string(),
            bytes: &TEST_ORACLE_WASM_BYTES,
            signer_account: near,
            deposit: to_yocto("10")
        );

        let contract = deploy!(
            contract: BurrowlandContract,
            contract_id: BURROWLAND_ID.to_string(),
            bytes: &BURROWLAND_WASM_BYTES,
            signer_account: near,
            deposit: to_yocto("20"),
            gas: DEFAULT_GAS,
            init_method: new(
                Config {
                    oracle_account_id: ORACLE_ID.to_string().try_into().unwrap(),
                    owner_id: owner.valid_account_id(),
                    booster_token_id: BOOSTER_TOKEN_ID.to_string(),
                    booster_decimals: BOOSTER_TOKEN_DECIMALS,
                }
            )
        );

        let booster_token = contract.user_account.deploy_and_init(
            &FUNGIBLE_TOKEN_WASM_BYTES,
            BOOSTER_TOKEN_ID.to_string(),
            "new",
            &json!({
                "owner_id": owner.valid_account_id(),
                "total_supply": U128::from(BOOSTER_TOKEN_TOTAL_SUPPLY),
                "metadata": FungibleTokenMetadata {
                    spec: FT_METADATA_SPEC.to_string(),
                    name: "Booster Token".to_string(),
                    symbol: "BOOSTER".to_string(),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals: BOOSTER_TOKEN_DECIMALS,
                }
            })
            .to_string()
            .into_bytes(),
            to_yocto("10"),
            DEFAULT_GAS,
        );

        storage_deposit(&owner, BOOSTER_TOKEN_ID, BURROWLAND_ID);

        Self {
            root,
            near,
            owner,
            contract,
            oracle,
            booster_token,
        }
    }

    pub fn setup_assets(&self, tokens: &Tokens) {
        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    self.booster_token.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000008319516250272147),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 2000,
                        extra_decimals: 0,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: false,
                        can_borrow: false,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();

        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    tokens.neth.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000001547125956667610),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 6000,
                        extra_decimals: 0,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: true,
                        can_borrow: true,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();

        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    tokens.ndai.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000002440418605283556),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 9500,
                        extra_decimals: 0,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: true,
                        can_borrow: true,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();

        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    tokens.nusdt.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000002440418605283556),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 9500,
                        extra_decimals: 12,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: true,
                        can_borrow: true,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();

        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    tokens.nusdc.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000002440418605283556),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 9500,
                        extra_decimals: 12,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: true,
                        can_borrow: true,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();

        self.owner
            .function_call(
                self.contract.contract.add_asset(
                    tokens.wnear.valid_account_id(),
                    AssetConfig {
                        reserve_ratio: 2500,
                        target_utilization: 8000,
                        target_utilization_rate: U128(1000000000003593629036885046),
                        max_utilization_rate: U128(1000000000039724853136740579),
                        volatility_ratio: 6000,
                        extra_decimals: 0,
                        can_deposit: true,
                        can_withdraw: true,
                        can_use_as_collateral: true,
                        can_borrow: true,
                    },
                ),
                DEFAULT_GAS,
                ONE_YOCTO,
            )
            .assert_success();
    }

    pub fn deposit_reserves(&self, tokens: &Tokens) {
        self.contract_ft_transfer_call(
            &tokens.wnear,
            &self.owner,
            10000 * 10u128.pow(24),
            DEPOSIT_TO_RESERVE.to_string(),
        );
        self.contract_ft_transfer_call(
            &tokens.neth,
            &self.owner,
            10000 * 10u128.pow(18),
            DEPOSIT_TO_RESERVE.to_string(),
        );
        self.contract_ft_transfer_call(
            &tokens.ndai,
            &self.owner,
            10000 * 10u128.pow(18),
            DEPOSIT_TO_RESERVE.to_string(),
        );
        self.contract_ft_transfer_call(
            &tokens.nusdt,
            &self.owner,
            10000 * 10u128.pow(6),
            DEPOSIT_TO_RESERVE.to_string(),
        );
        self.contract_ft_transfer_call(
            &tokens.nusdc,
            &self.owner,
            10000 * 10u128.pow(6),
            DEPOSIT_TO_RESERVE.to_string(),
        );
        self.contract_ft_transfer_call(
            &self.booster_token,
            &self.owner,
            10000 * 10u128.pow(18),
            DEPOSIT_TO_RESERVE.to_string(),
        );
    }

    pub fn contract_ft_transfer_call(
        &self,
        token: &UserAccount,
        user: &UserAccount,
        amount: Balance,
        msg: String,
    ) -> ExecutionResult {
        user.call(
            token.account_id.clone(),
            "ft_transfer_call",
            &json!({
                "receiver_id": self.contract.user_account.valid_account_id(),
                "amount": U128::from(amount),
                "msg": msg,
            })
            .to_string()
            .into_bytes(),
            MAX_GAS,
            1,
        )
    }

    pub fn mint_ft(&self, token: &UserAccount, receiver: &UserAccount, amount: Balance) {
        self.owner
            .call(
                token.account_id.clone(),
                "ft_transfer",
                &json!({
                    "receiver_id": receiver.valid_account_id(),
                    "amount": U128::from(amount),
                })
                .to_string()
                .into_bytes(),
                DEFAULT_GAS,
                1,
            )
            .assert_success();
    }

    pub fn mint_tokens(&self, tokens: &Tokens, user: &UserAccount) {
        storage_deposit(user, &tokens.wnear.account_id(), &user.account_id());
        storage_deposit(user, &tokens.neth.account_id(), &user.account_id());
        storage_deposit(user, &tokens.ndai.account_id(), &user.account_id());
        storage_deposit(user, &tokens.nusdt.account_id(), &user.account_id());
        storage_deposit(user, &tokens.nusdc.account_id(), &user.account_id());
        storage_deposit(user, &self.booster_token.account_id(), &user.account_id());

        self.mint_ft(&tokens.wnear, user, 10000 * 10u128.pow(24));
        self.mint_ft(&tokens.neth, user, 10000 * 10u128.pow(18));
        self.mint_ft(&tokens.ndai, user, 10000 * 10u128.pow(18));
        self.mint_ft(&tokens.nusdt, user, 10000 * 10u128.pow(6));
        self.mint_ft(&tokens.nusdc, user, 10000 * 10u128.pow(6));
        self.mint_ft(&self.booster_token, user, 10000 * 10u128.pow(18));
    }
}

pub fn init_token(e: &Env, token_account_id: &str, decimals: u8) -> UserAccount {
    let token = e.near.deploy_and_init(
        &FUNGIBLE_TOKEN_WASM_BYTES,
        token_account_id.to_string(),
        "new",
        &json!({
            "owner_id": e.owner.valid_account_id(),
            "total_supply": U128::from(10u128.pow((9 + decimals) as _)),
            "metadata": FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: token_account_id.to_string(),
                symbol: token_account_id.to_string(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: decimals,
            }
        })
        .to_string()
        .into_bytes(),
        to_yocto("10"),
        DEFAULT_GAS,
    );

    storage_deposit(&e.owner, token_account_id, BURROWLAND_ID);
    token
}

impl Tokens {
    pub fn init(e: &Env) -> Self {
        Self {
            wnear: init_token(e, "wrap.near", 24),
            neth: init_token(e, "neth.near", 18),
            ndai: init_token(e, "dai.near", 18),
            nusdt: init_token(e, "nusdt.near", 6),
            nusdc: init_token(e, "nusdc.near", 6),
        }
    }
}

impl Users {
    pub fn init(e: &Env) -> Self {
        Self {
            alice: e
                .near
                .create_user("alice.near".to_string(), to_yocto("10000")),
            bob: e
                .near
                .create_user("bob.near".to_string(), to_yocto("10000")),
            charlie: e
                .near
                .create_user("charlie.near".to_string(), to_yocto("10000")),
            dude: e
                .near
                .create_user("dude.near".to_string(), to_yocto("10000")),
            eve: e
                .near
                .create_user("eve.near".to_string(), to_yocto("10000")),
        }
    }
}
