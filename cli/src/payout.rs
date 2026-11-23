//! Builds the hidden payout statement that the tacita template's `pay` method checks.

use std::num::NonZeroU64;

use ootle_rs::{
    key_provider::PrivateKeyProvider,
    stealth::{Output, ResolvedStealthTransferSpec, StealthResult, StealthStatementProvider},
    template_types::{constants::TARI_TOKEN, stealth::StealthTransferStatement, Amount},
    Address,
};

/// One line of a campaign: who gets paid and how much (in µTARI).
pub struct Line {
    pub address: Address,
    pub amount: NonZeroU64,
}

/// Builds a statement that takes exactly `sum(lines)` from the campaign vault as a revealed
/// input and turns it into one hidden output per line, with nothing revealed on the way out.
pub async fn build_statement(payer: &PrivateKeyProvider, lines: &[Line]) -> StealthResult<StealthTransferStatement> {
    let total: u64 = lines.iter().map(|l| l.amount.get()).sum();
    let outputs = lines
        .iter()
        .map(|l| Output::new(l.address.clone(), TARI_TOKEN, l.amount))
        .collect();
    let spec = ResolvedStealthTransferSpec {
        inputs: vec![],
        revealed_input_amount: Amount::from(total),
        outputs,
        revealed_output: None,
    };
    payer.create_transfer_statement(spec).await
}

/// The same rules the template enforces on chain, checked locally before anything is sent.
pub fn check_statement(statement: &StealthTransferStatement, headcount: usize, total: u64) -> Result<(), String> {
    if statement.stealth_outputs().len() != headcount {
        return Err(format!("expected {headcount} hidden outputs, got {}", statement.stealth_outputs().len()));
    }
    for o in statement.stealth_outputs() {
        if o.output.viewable_balance_proof.is_some() {
            return Err("an output is viewable".into());
        }
        if o.output.minimum_value_promise != 0 {
            return Err("an output promises a minimum value".into());
        }
    }
    if statement.revealed_output_amount() != Amount::zero() {
        return Err("something would be paid out in the clear".into());
    }
    if statement.revealed_input_amount() != Amount::from(total) {
        return Err("payout does not equal the round total".into());
    }
    if statement.balance_proof.is_none() {
        return Err("missing balance proof".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ootle_rs::{transaction::TransactionSigner, Network};

    use super::*;

    fn line(amount: u64) -> Line {
        Line {
            address: PrivateKeyProvider::random(Network::MainNet).address().clone(),
            amount: NonZeroU64::new(amount).unwrap(),
        }
    }

    #[tokio::test]
    async fn three_winner_statement_passes_the_template_rules() {
        let payer = PrivateKeyProvider::random(Network::MainNet);
        let lines = vec![line(300), line(250), line(350)];
        let statement = build_statement(&payer, &lines).await.unwrap();
        check_statement(&statement, 3, 900).unwrap();
        let args = tari_ootle_transaction::args![statement];
        assert_eq!(args.len(), 1);
    }

    #[tokio::test]
    async fn wrong_headcount_or_total_is_rejected() {
        let payer = PrivateKeyProvider::random(Network::MainNet);
        let lines = vec![line(100), line(200)];
        let statement = build_statement(&payer, &lines).await.unwrap();
        assert!(check_statement(&statement, 3, 300).is_err());
        assert!(check_statement(&statement, 2, 301).is_err());
        assert!(check_statement(&statement, 2, 300).is_ok());
    }
}
