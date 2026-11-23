//! builds the instruction lists tacita hands to the wallet.
//!
//! the wallet (walletd) adds the fee, signs with your account key and submits.
//! tacita only decides *what* the transaction does.

use ootle_rs::template_types::{
    constants::TARI_TOKEN, stealth::StealthTransferStatement, Amount, ComponentAddress, Hash32,
    TemplateAddress,
};
use serde_json::Value;
use tari_ootle_transaction::{args, Epoch, Transaction};

fn to_json(tx: tari_ootle_transaction::UnsignedTransaction) -> Result<Value, String> {
    serde_json::to_value(tx.instructions()).map_err(|e| format!("could not encode instructions: {e}"))
}

// the builder needs a network and an epoch, but we only keep the instruction list.
// the wallet sets the real network, epoch window and fee when it submits.
fn builder() -> tari_ootle_transaction::TransactionBuilder {
    Transaction::builder(0u8, Epoch(u64::MAX))
}

/// take `total` revealed TARI out of the account and lock it in a new campaign.
pub fn create(
    account: ComponentAddress,
    template: TemplateAddress,
    name: &str,
    total: Amount,
    headcount: u32,
    manifest_root: Hash32,
) -> Result<Value, String> {
    let tx = builder()
        .call_method(account, "withdraw", args![TARI_TOKEN, total])
        .put_last_instruction_output_on_workspace("funding")
        .call_function(template, "new", args![
            name.to_string(),
            total,
            headcount,
            manifest_root,
            Workspace("funding")
        ])
        .build_unsigned();
    to_json(tx)
}

/// pay every winner from the campaign vault with one hidden-amount statement.
pub fn pay(campaign: ComponentAddress, statement: StealthTransferStatement) -> Result<Value, String> {
    let tx = builder().call_method(campaign, "pay", args![statement]).build_unsigned();
    to_json(tx)
}

/// return whatever is left in the campaign to the owner's account.
pub fn close(campaign: ComponentAddress, account: ComponentAddress) -> Result<Value, String> {
    let tx = builder()
        .call_method(campaign, "close", args![])
        .put_last_instruction_output_on_workspace("leftover")
        .call_method(account, "deposit", args![Workspace("leftover")])
        .build_unsigned();
    to_json(tx)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    const ACCOUNT: &str = "component_5286446acff808c0fcad494ce7b74e19238b446b02394242328d974261d3fe36";

    #[test]
    fn create_encodes_three_instructions() {
        let account = ComponentAddress::from_str(ACCOUNT).unwrap();
        let v = create(account, TemplateAddress::default(), "test round", Amount::from(900u64), 3, Hash32::zero()).unwrap();
        let list = v.as_array().unwrap();
        assert_eq!(list.len(), 3, "{v}");
        let text = v.to_string();
        assert!(text.contains("withdraw") && text.contains("\"new\""), "{text}");
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }

    #[test]
    fn close_deposits_back() {
        let account = ComponentAddress::from_str(ACCOUNT).unwrap();
        let v = close(account, account).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }
}
