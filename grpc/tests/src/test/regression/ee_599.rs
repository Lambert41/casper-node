use lazy_static::lazy_static;

use casperlabs_engine_test_support::{
    internal::{
        utils, ExecuteRequestBuilder, InMemoryWasmTestBuilder, DEFAULT_PAYMENT,
        DEFAULT_RUN_GENESIS_REQUEST,
    },
    DEFAULT_ACCOUNT_ADDR,
};
use casperlabs_node::{components::contract_runtime::core::engine_state::CONV_RATE, types::Motes};
use casperlabs_types::{account::AccountHash, runtime_args, RuntimeArgs, U512};

const CONTRACT_EE_599_REGRESSION: &str = "ee_599_regression.wasm";
const CONTRACT_TRANSFER_TO_ACCOUNT: &str = "transfer_to_account_u512.wasm";
const DONATION_PURSE_COPY_KEY: &str = "donation_purse_copy";
const EXPECTED_ERROR: &str = "InvalidContext";
const TRANSFER_FUNDS_KEY: &str = "transfer_funds";
const VICTIM_ADDR: AccountHash = AccountHash::new([42; 32]);

lazy_static! {
    static ref VICTIM_INITIAL_FUNDS: U512 = *DEFAULT_PAYMENT * 10;
}

fn setup() -> InMemoryWasmTestBuilder {
    // Creates victim account
    let exec_request_1 = {
        let args = runtime_args! {
            "target" => VICTIM_ADDR,
            "amount" => *VICTIM_INITIAL_FUNDS,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, CONTRACT_TRANSFER_TO_ACCOUNT, args)
            .build()
    };

    // Deploy contract
    let exec_request_2 = {
        let args = runtime_args! {
            "method" => "install".to_string(),
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, CONTRACT_EE_599_REGRESSION, args)
            .build()
    };

    let result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_RUN_GENESIS_REQUEST)
        .exec(exec_request_1)
        .expect_success()
        .commit()
        .exec(exec_request_2)
        .expect_success()
        .commit()
        .finish();

    InMemoryWasmTestBuilder::from_result(result)
}

#[ignore]
#[test]
fn should_not_be_able_to_transfer_funds_with_transfer_purse_to_purse() {
    let mut builder = setup();

    let victim_account = builder
        .get_account(VICTIM_ADDR)
        .expect("should have victim account");

    let default_account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");
    let transfer_funds = default_account
        .named_keys()
        .get(TRANSFER_FUNDS_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", TRANSFER_FUNDS_KEY));
    let donation_purse_copy_key = default_account
        .named_keys()
        .get(DONATION_PURSE_COPY_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", DONATION_PURSE_COPY_KEY));

    let donation_purse_copy = donation_purse_copy_key.into_uref().expect("should be uref");

    let exec_request_3 = {
        let args = runtime_args! {
            "method" => "call",
            "contract_key" => transfer_funds.into_hash().expect("should be hash"),
            "sub_contract_method_fwd" => "transfer_from_purse_to_purse_ext",
        };
        ExecuteRequestBuilder::standard(VICTIM_ADDR, CONTRACT_EE_599_REGRESSION, args).build()
    };

    let result_2 = builder.exec(exec_request_3).commit().finish();

    let exec_3_response = result_2
        .builder()
        .get_exec_response(0)
        .expect("should have response");
    let gas_cost = Motes::from_gas(utils::get_exec_costs(exec_3_response)[0], CONV_RATE)
        .expect("should convert");

    let error_msg = result_2
        .builder()
        .exec_error_message(0)
        .expect("should have error");
    assert!(
        error_msg.contains(EXPECTED_ERROR),
        "Got error: {}",
        error_msg
    );

    let victim_balance_after = result_2
        .builder()
        .get_purse_balance(victim_account.main_purse());

    assert_eq!(
        *VICTIM_INITIAL_FUNDS - gas_cost.value(),
        victim_balance_after
    );

    assert_eq!(
        result_2.builder().get_purse_balance(donation_purse_copy),
        U512::zero(),
    );
}

#[ignore]
#[test]
fn should_not_be_able_to_transfer_funds_with_transfer_from_purse_to_account() {
    let mut builder = setup();

    let victim_account = builder
        .get_account(VICTIM_ADDR)
        .expect("should have victim account");

    let default_account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let default_account_balance = builder.get_purse_balance(default_account.main_purse());

    let transfer_funds = default_account
        .named_keys()
        .get(TRANSFER_FUNDS_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", TRANSFER_FUNDS_KEY));
    let donation_purse_copy_key = default_account
        .named_keys()
        .get(DONATION_PURSE_COPY_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", DONATION_PURSE_COPY_KEY));

    let donation_purse_copy = donation_purse_copy_key.into_uref().expect("should be uref");

    let exec_request_3 = {
        let args = runtime_args! {
            "method" => "call".to_string(),
            "contract_key" => transfer_funds.into_hash().expect("should get key"),
            "sub_contract_method_fwd" => "transfer_from_purse_to_account_ext",
        };
        ExecuteRequestBuilder::standard(VICTIM_ADDR, CONTRACT_EE_599_REGRESSION, args).build()
    };

    let result_2 = builder.exec(exec_request_3).commit().finish();

    let exec_3_response = result_2
        .builder()
        .get_exec_response(0)
        .expect("should have response");

    let gas_cost = Motes::from_gas(utils::get_exec_costs(exec_3_response)[0], CONV_RATE)
        .expect("should convert");

    let error_msg = result_2
        .builder()
        .exec_error_message(0)
        .expect("should have error");
    assert!(
        error_msg.contains(EXPECTED_ERROR),
        "Got error: {}",
        error_msg
    );

    let victim_balance_after = result_2
        .builder()
        .get_purse_balance(victim_account.main_purse());

    assert_eq!(
        *VICTIM_INITIAL_FUNDS - gas_cost.value(),
        victim_balance_after
    );
    // In this variant of test `donation_purse` is left unchanged i.e. zero balance
    assert_eq!(
        result_2.builder().get_purse_balance(donation_purse_copy),
        U512::zero(),
    );

    // Main purse of the contract owner is unchanged
    let updated_default_account_balance = result_2
        .builder()
        .get_purse_balance(default_account.main_purse());

    assert_eq!(
        updated_default_account_balance - default_account_balance,
        U512::zero(),
    )
}

#[ignore]
#[test]
fn should_not_be_able_to_transfer_funds_with_transfer_to_account() {
    let mut builder = setup();

    let victim_account = builder
        .get_account(VICTIM_ADDR)
        .expect("should have victim account");

    let default_account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let default_account_balance = builder.get_purse_balance(default_account.main_purse());

    let transfer_funds = default_account
        .named_keys()
        .get(TRANSFER_FUNDS_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", TRANSFER_FUNDS_KEY));
    let donation_purse_copy_key = default_account
        .named_keys()
        .get(DONATION_PURSE_COPY_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", DONATION_PURSE_COPY_KEY));

    let donation_purse_copy = donation_purse_copy_key.into_uref().expect("should be uref");

    let exec_request_3 = {
        let args = runtime_args! {
            "method" => "call",
            "contract_key" => transfer_funds.into_hash().expect("should be hash"),
            "sub_contract_method_fwd" => "transfer_to_account_ext",
        };
        ExecuteRequestBuilder::standard(VICTIM_ADDR, CONTRACT_EE_599_REGRESSION, args).build()
    };

    let result_2 = builder.exec(exec_request_3).commit().finish();

    let exec_3_response = result_2
        .builder()
        .get_exec_response(0)
        .expect("should have response");

    let gas_cost = Motes::from_gas(utils::get_exec_costs(exec_3_response)[0], CONV_RATE)
        .expect("should convert");

    let error_msg = result_2
        .builder()
        .exec_error_message(0)
        .expect("should have error");
    assert!(
        error_msg.contains(EXPECTED_ERROR),
        "Got error: {}",
        error_msg
    );

    let victim_balance_after = result_2
        .builder()
        .get_purse_balance(victim_account.main_purse());

    assert_eq!(
        *VICTIM_INITIAL_FUNDS - gas_cost.value(),
        victim_balance_after
    );

    // In this variant of test `donation_purse` is left unchanged i.e. zero balance
    assert_eq!(
        result_2.builder().get_purse_balance(donation_purse_copy),
        U512::zero(),
    );

    // Verify that default account's balance didn't change
    let updated_default_account_balance = result_2
        .builder()
        .get_purse_balance(default_account.main_purse());

    assert_eq!(
        updated_default_account_balance - default_account_balance,
        U512::zero(),
    )
}

#[ignore]
#[test]
fn should_not_be_able_to_get_main_purse_in_invalid_context() {
    let mut builder = setup();

    let victim_account = builder
        .get_account(VICTIM_ADDR)
        .expect("should have victim account");

    let default_account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let transfer_funds = default_account
        .named_keys()
        .get(TRANSFER_FUNDS_KEY)
        .cloned()
        .unwrap_or_else(|| panic!("should have {}", TRANSFER_FUNDS_KEY));

    let exec_request_3 = {
        let args = runtime_args! {
            "method" => "call".to_string(),
            "contract_key" => transfer_funds.into_hash().expect("should be hash"),
            "sub_contract_method_fwd" => "transfer_to_account_ext",
        };
        ExecuteRequestBuilder::standard(VICTIM_ADDR, CONTRACT_EE_599_REGRESSION, args).build()
    };

    let victim_balance_before = builder.get_purse_balance(victim_account.main_purse());

    let result_2 = builder.exec(exec_request_3).commit().finish();

    let exec_3_response = result_2
        .builder()
        .get_exec_response(0)
        .expect("should have response");

    let gas_cost = Motes::from_gas(utils::get_exec_costs(exec_3_response)[0], CONV_RATE)
        .expect("should convert");

    let error_msg = result_2
        .builder()
        .exec_error_message(0)
        .expect("should have error");
    assert!(
        error_msg.contains(EXPECTED_ERROR),
        "Got error: {}",
        error_msg
    );

    let victim_balance_after = result_2
        .builder()
        .get_purse_balance(victim_account.main_purse());

    assert_eq!(
        victim_balance_before - gas_cost.value(),
        victim_balance_after
    );
}
