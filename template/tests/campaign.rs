//! engine tests for the tacita campaign template.
//!
//! each test compiles the template to wasm and runs real transactions through the ootle engine,
//! including the stealth transfer that pays the winners. these test the code exactly as it is
//! published on mainnet (template_7d80993c…bc46).

use tari_template_lib::prelude::*;
use tari_template_test_tooling::{
    crypto::RistrettoSecretKey,
    support::stealth::generate_mint_statement,
    transaction::args,
    TemplateTest,
};
use tari_template_test_tooling::template_lib_types::NonFungibleAddress;

const AMOUNTS: [u64; 3] = [300_000, 250_000, 450_000];
const TOTAL: u64 = 1_000_000;

struct World {
    test: TemplateTest,
    account: ComponentAddress,
    proof: NonFungibleAddress,
    key: RistrettoSecretKey,
}

fn world() -> World {
    let mut test = TemplateTest::my_crate();
    let (account, proof, key) = test.create_funded_account();
    World { test, account, proof, key }
}

fn root() -> Hash32 {
    Hash32::from_array([7u8; 32])
}

impl World {
    /// creates a campaign funded with `funding` from the owner's account and returns its address.
    fn try_create(&mut self, name: &str, total: u64, headcount: u32, funding: u64) -> Result<ComponentAddress, String> {
        let template = self.test.get_template_address("Campaign");
        let tx = self
            .test
            .transaction()
            .call_method(self.account, "withdraw", args![TARI_TOKEN, Amount::from(funding)])
            .put_last_instruction_output_on_workspace("funding")
            .call_function(template, "new", args![
                name.to_string(),
                Amount::from(total),
                headcount,
                root(),
                Workspace("funding")
            ])
            .build_and_seal(&self.key);
        let result = self.test.execute_expect_commit(tx, vec![self.proof.clone()]);
        if let Some(reason) = result.finalize.any_reject() {
            return Err(format!("{reason:?}"));
        }
        let account = self.account;
        self.test
            .return_values()
            .iter()
            .filter_map(|v| v.decoded::<ComponentAddress>().ok())
            .find(|c| *c != account)
            .ok_or_else(|| "no campaign address returned".to_string())
    }

    fn create(&mut self) -> ComponentAddress {
        self.try_create("round 1", TOTAL, AMOUNTS.len() as u32, TOTAL).expect("create")
    }

    /// runs `method` on the campaign signed by `key`; Ok on success, Err(reason) on rejection.
    fn call(&mut self, campaign: ComponentAddress, method: &str, a: Vec<tari_template_test_tooling::transaction::builder::named_args::NamedArg>, key: &RistrettoSecretKey) -> Result<(), String> {
        let tx = self.test.transaction().call_method(campaign, method, a).build_and_seal(key);
        let result = self.test.execute_expect_commit(tx, vec![]);
        match result.finalize.any_reject() {
            Some(reason) => Err(format!("{reason:?}")),
            None => Ok(()),
        }
    }

    fn pay_with(&mut self, campaign: ComponentAddress, amounts: &[u64], revealed_out: u64, view: bool) -> Result<(), String> {
        let view_key = view.then(|| self.test.new_key_pair(200).1);
        let data = generate_mint_statement(amounts.iter().copied(), revealed_out, view_key.as_ref());
        let key = self.key.clone();
        self.call(campaign, "pay", args![data.statement], &key)
    }

    fn summary(&mut self, campaign: ComponentAddress) -> (String, Amount, u32, Hash32, bool, bool, Amount) {
        self.test.call_method(campaign, "summary", args![], vec![])
    }
}

fn assert_rejected(r: Result<(), String>, needle: &str) {
    match r {
        Ok(()) => panic!("expected a rejection containing {needle:?}, but it succeeded"),
        Err(e) => assert!(e.contains(needle), "expected {needle:?} in: {e}"),
    }
}

// ---------- creating a campaign ----------

#[test]
fn create_locks_the_total_and_records_the_round() {
    let mut w = world();
    let c = w.create();
    let (name, total, headcount, manifest_root, paid, closed, balance) = w.summary(c);
    assert_eq!(name, "round 1");
    assert_eq!(total, Amount::from(TOTAL));
    assert_eq!(headcount, 3);
    assert_eq!(manifest_root, root());
    assert!(!paid && !closed);
    assert_eq!(balance, Amount::from(TOTAL));
}

#[test]
fn create_refuses_zero_winners() {
    let mut w = world();
    let e = w.try_create("r", TOTAL, 0, TOTAL).unwrap_err();
    assert!(e.contains("headcount must be at least 1"), "{e}");
}

#[test]
fn create_refuses_more_than_sixteen_winners() {
    let mut w = world();
    let e = w.try_create("r", TOTAL, 17, TOTAL).unwrap_err();
    assert!(e.contains("at most 16 winners"), "{e}");
}

#[test]
fn create_accepts_exactly_sixteen_winners() {
    let mut w = world();
    assert!(w.try_create("r", TOTAL, 16, TOTAL).is_ok());
}

#[test]
fn create_refuses_a_zero_total() {
    let mut w = world();
    let e = w.try_create("r", 0, 3, TOTAL).unwrap_err();
    assert!(e.contains("total must be positive"), "{e}");
}

#[test]
fn create_refuses_underfunding() {
    let mut w = world();
    let e = w.try_create("r", TOTAL, 3, TOTAL - 1).unwrap_err();
    assert!(e.contains("funding is less than the round total"), "{e}");
}

#[test]
fn create_refuses_a_long_name() {
    let mut w = world();
    let e = w.try_create(&"x".repeat(81), TOTAL, 3, TOTAL).unwrap_err();
    assert!(e.contains("name too long"), "{e}");
}

// ---------- paying ----------

#[test]
fn pay_sends_one_hidden_output_per_winner_and_empties_the_vault() {
    let mut w = world();
    let c = w.create();
    w.pay_with(c, &AMOUNTS, 0, false).expect("pay");
    let (_, _, _, _, paid, closed, balance) = w.summary(c);
    assert!(paid && !closed);
    assert_eq!(balance, Amount::zero());
}

#[test]
fn rule_1_only_the_owner_can_pay() {
    let mut w = world();
    let c = w.create();
    let data = generate_mint_statement(AMOUNTS, 0u64, None);
    let stranger = w.test.new_key_pair(201).0;
    assert_rejected(w.call(c, "pay", args![data.statement], &stranger), "only the campaign owner");
}

#[test]
fn rule_2_a_round_can_only_be_paid_once() {
    let mut w = world();
    let c = w.create();
    w.pay_with(c, &AMOUNTS, 0, false).expect("first pay");
    assert_rejected(w.pay_with(c, &AMOUNTS, 0, false), "already paid");
}

#[test]
fn rule_3_too_few_outputs_are_refused() {
    let mut w = world();
    let c = w.create();
    assert_rejected(w.pay_with(c, &[500_000, 500_000], 0, false), "exactly one hidden output per winner");
}

#[test]
fn rule_3_too_many_outputs_are_refused() {
    let mut w = world();
    let c = w.create();
    assert_rejected(w.pay_with(c, &[250_000; 4], 0, false), "exactly one hidden output per winner");
}

#[test]
fn rule_4_viewable_outputs_are_refused() {
    let mut w = world();
    let c = w.create();
    assert_rejected(w.pay_with(c, &AMOUNTS, 0, true), "viewable on chain");
}

#[test]
fn rule_6_paying_out_in_the_clear_is_refused() {
    let mut w = world();
    // the statement's revealed input is the outputs plus the revealed part, so the total matches
    // and only the "nothing in the clear" rule can stop it
    let c = w.try_create("r", TOTAL + 100_000, 3, TOTAL + 100_000).unwrap();
    assert_rejected(w.pay_with(c, &AMOUNTS, 100_000, false), "nothing may be paid out in the clear");
}

#[test]
fn rule_7_paying_less_than_the_total_is_refused() {
    let mut w = world();
    let c = w.create();
    assert_rejected(w.pay_with(c, &[300_000, 250_000, 449_999], 0, false), "payout must equal the round total");
}

#[test]
fn rule_7_paying_more_than_the_total_is_refused() {
    let mut w = world();
    let c = w.try_create("r", TOTAL, 3, TOTAL + 500_000).unwrap();
    assert_rejected(w.pay_with(c, &[300_000, 250_000, 450_001], 0, false), "payout must equal the round total");
}

#[test]
fn a_refused_payout_leaves_the_funds_locked() {
    let mut w = world();
    let c = w.create();
    let _ = w.pay_with(c, &[500_000, 500_000], 0, false);
    let (_, _, _, _, paid, _, balance) = w.summary(c);
    assert!(!paid);
    assert_eq!(balance, Amount::from(TOTAL));
}

// ---------- closing ----------

#[test]
fn close_returns_unpaid_funds_to_the_owner() {
    let mut w = world();
    let c = w.create();
    let tx = w
        .test
        .transaction()
        .call_method(c, "close", args![])
        .put_last_instruction_output_on_workspace("left")
        .call_method(w.account, "deposit", args![Workspace("left")])
        .build_and_seal(&w.key);
    w.test.execute_expect_success(tx, vec![w.proof.clone()]);
    let (_, _, _, _, paid, closed, balance) = w.summary(c);
    assert!(!paid && closed);
    assert_eq!(balance, Amount::zero());
}

#[test]
fn only_the_owner_can_close() {
    let mut w = world();
    let c = w.create();
    let stranger = w.test.new_key_pair(202).0;
    assert_rejected(w.call(c, "close", args![], &stranger), "only the campaign owner");
}

#[test]
fn a_closed_campaign_cannot_pay() {
    let mut w = world();
    let c = w.create();
    let tx = w
        .test
        .transaction()
        .call_method(c, "close", args![])
        .put_last_instruction_output_on_workspace("left")
        .call_method(w.account, "deposit", args![Workspace("left")])
        .build_and_seal(&w.key);
    w.test.execute_expect_success(tx, vec![w.proof.clone()]);
    assert_rejected(w.pay_with(c, &AMOUNTS, 0, false), "campaign is closed");
}
