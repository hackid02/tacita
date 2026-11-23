//! tacita: private campaign payouts on Tari Ootle.
//!
//! A campaign holds a public total, a public number of winners and a manifest
//! root (a hash with no names in it). `pay` accepts a stealth transfer
//! statement built off-chain and refuses it unless every rule holds:
//!
//! 1. only the campaign owner can pay (checked through the transaction signer)
//! 2. a round can only be paid once
//! 3. exactly one hidden output per winner
//! 4. no output carries a viewable balance proof, so no amount is readable on chain
//! 5. no output promises a minimum value
//! 6. nothing is paid out in the clear (revealed output is zero)
//! 7. the amount taken from the vault equals the announced round total
//!
//! The balance proof itself is checked by the engine when the stealth
//! transfer runs.

use tari_template_lib::prelude::*;

/// Ootle allows at most 16 stealth outputs in one transfer.
pub const MAX_WINNERS: u32 = 16;

#[template]
mod tacita {
    use super::*;

    pub struct Campaign {
        owner: RistrettoPublicKeyBytes,
        name: String,
        total: Amount,
        headcount: u32,
        manifest_root: Hash32,
        vault: Vault,
        paid: bool,
        closed: bool,
    }

    impl Campaign {
        /// Creates a campaign and funds it in one step.
        pub fn new(
            name: String,
            total: Amount,
            headcount: u32,
            manifest_root: Hash32,
            funding: Bucket,
        ) -> Component<Self> {
            assert!(headcount > 0, "tacita: headcount must be at least 1");
            assert!(headcount <= MAX_WINNERS, "tacita: at most 16 winners per round");
            assert!(total > Amount::zero(), "tacita: total must be positive");
            assert!(name.len() <= 80, "tacita: name too long");
            assert!(funding.amount() >= total, "tacita: funding is less than the round total");

            let owner = CallerContext::transaction_signer_public_key();
            Component::new(Self {
                owner,
                name,
                total,
                headcount,
                manifest_root,
                vault: Vault::from_bucket(funding),
                paid: false,
                closed: false,
            })
            .with_access_rules(ComponentAccessRules::new().default(rule!(allow_all)))
            .create()
        }

        /// Pays every winner of the round in one stealth transfer.
        pub fn pay(&mut self, statement: StealthTransferStatement) {
            self.require_owner();
            assert!(!self.closed, "tacita: campaign is closed");
            assert!(!self.paid, "tacita: round already paid");

            let outputs = statement.stealth_outputs();
            assert_eq!(
                outputs.len(),
                self.headcount as usize,
                "tacita: need exactly one hidden output per winner"
            );
            for out in outputs {
                assert!(
                    out.output.viewable_balance_proof.is_none(),
                    "tacita: an output is viewable on chain"
                );
                assert_eq!(
                    out.output.minimum_value_promise, 0,
                    "tacita: outputs may not promise a minimum value"
                );
            }
            assert_eq!(
                statement.revealed_output_amount(),
                Amount::zero(),
                "tacita: nothing may be paid out in the clear"
            );
            let revealed_in = statement.revealed_input_amount();
            assert_eq!(revealed_in, self.total, "tacita: payout must equal the round total");

            let input = self.vault.withdraw(revealed_in);
            let rm = ResourceManager::get(self.vault.resource_address());
            if let Some(change) = rm.stealth_transfer_with_opt_input_bucket(statement, Some(input)) {
                // anything left over goes back to the campaign vault, never to the operator
                self.vault.deposit(change);
            }
            self.paid = true;

            emit_event("tacita.paid", [
                ("headcount".to_string(), self.headcount.to_string()),
                ("total".to_string(), self.total.to_string()),
            ]);
        }

        /// Returns whatever is left in the vault to the owner and closes the campaign.
        pub fn close(&mut self) -> Bucket {
            self.require_owner();
            assert!(!self.closed, "tacita: campaign already closed");
            self.closed = true;
            let left = self.vault.balance();
            self.vault.withdraw(left)
        }

        /// Public view: name, total, headcount, manifest root, paid, closed, balance.
        pub fn summary(&self) -> (String, Amount, u32, Hash32, bool, bool, Amount) {
            (
                self.name.clone(),
                self.total,
                self.headcount,
                self.manifest_root,
                self.paid,
                self.closed,
                self.vault.balance(),
            )
        }

        fn require_owner(&self) {
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                self.owner,
                "tacita: only the campaign owner can do this"
            );
        }
    }
}
