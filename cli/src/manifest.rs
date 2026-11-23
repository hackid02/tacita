//! the winners file: one `address,amount` per line, amounts in TARI (up to 6 decimals).
//!
//! lines starting with `#` and blank lines are ignored. the manifest root is a sha256 over a
//! canonical form (`address,amount_in_micro_tari\n` per line, in file order), so anyone holding
//! the same file gets the same root and can check it against the one stored on chain.

use std::num::NonZeroU64;

use ootle_rs::Address;
use sha2::{Digest, Sha256};

use crate::payout::Line;

pub const MICRO: u64 = 1_000_000;

pub struct Manifest {
    pub lines: Vec<Line>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let row = raw.trim();
            if row.is_empty() || row.starts_with('#') {
                continue;
            }
            let (addr, amount) = row
                .split_once(',')
                .ok_or_else(|| format!("line {}: expected `address,amount`", n + 1))?;
            let address: Address = addr
                .trim()
                .parse()
                .map_err(|e| format!("line {}: bad address: {e}", n + 1))?;
            let micro = parse_tari(amount.trim()).map_err(|e| format!("line {}: {e}", n + 1))?;
            let amount = NonZeroU64::new(micro).ok_or_else(|| format!("line {}: amount must be above 0", n + 1))?;
            lines.push(Line { address, amount });
        }
        if lines.is_empty() {
            return Err("the winners file has no lines".into());
        }
        if lines.len() > 16 {
            return Err(format!("{} winners, but one round pays at most 16", lines.len()));
        }
        Ok(Self { lines })
    }

    pub fn total(&self) -> u64 {
        self.lines.iter().map(|l| l.amount.get()).sum()
    }

    pub fn headcount(&self) -> u32 {
        self.lines.len() as u32
    }

    pub fn root(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        for l in &self.lines {
            h.update(format!("{},{}\n", l.address, l.amount.get()).as_bytes());
        }
        h.finalize().into()
    }
}

pub fn parse_tari(s: &str) -> Result<u64, String> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty() && frac.is_empty() {
        return Err("empty amount".into());
    }
    if frac.len() > 6 {
        return Err(format!("`{s}` has more than 6 decimals"));
    }
    let whole: u64 = if whole.is_empty() { 0 } else { whole.parse().map_err(|_| format!("`{s}` is not a number"))? };
    let frac_micro: u64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<6}").parse().map_err(|_| format!("`{s}` is not a number"))?
    };
    whole
        .checked_mul(MICRO)
        .and_then(|w| w.checked_add(frac_micro))
        .ok_or_else(|| format!("`{s}` is too large"))
}

pub fn fmt_tari(micro: u64) -> String {
    let s = format!("{}.{:06}", micro / MICRO, micro % MICRO);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use ootle_rs::{key_provider::PrivateKeyProvider, transaction::TransactionSigner, Network};

    use super::*;

    #[test]
    fn amounts() {
        assert_eq!(parse_tari("1").unwrap(), 1_000_000);
        assert_eq!(parse_tari("0.25").unwrap(), 250_000);
        assert_eq!(parse_tari(".5").unwrap(), 500_000);
        assert_eq!(parse_tari("2.000001").unwrap(), 2_000_001);
        assert!(parse_tari("1.0000001").is_err());
        assert!(parse_tari("abc").is_err());
        assert_eq!(fmt_tari(2_500_000), "2.5");
        assert_eq!(fmt_tari(3_000_000), "3");
    }

    #[test]
    fn parses_and_roots_are_stable() {
        let a = PrivateKeyProvider::random(Network::MainNet).address().clone();
        let b = PrivateKeyProvider::random(Network::MainNet).address().clone();
        let text = format!("# round 1\n{a}, 1.5\n\n{b},0.5\n");
        let m = Manifest::parse(&text).unwrap();
        assert_eq!(m.headcount(), 2);
        assert_eq!(m.total(), 2_000_000);
        assert_eq!(m.root(), Manifest::parse(&text).unwrap().root());
        let swapped = format!("{b},0.5\n{a},1.5\n");
        assert_ne!(m.root(), Manifest::parse(&swapped).unwrap().root());
    }
}

