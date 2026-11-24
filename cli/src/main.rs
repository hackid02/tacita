mod fund;
mod manifest;
mod payout;
mod tx;
mod walletd;

use std::{collections::HashMap, env, fs, str::FromStr};

use manifest::{fmt_tari, hex, Manifest};
use ootle_rs::{
    key_provider::PrivateKeyProvider,
    template_types::{Amount, ComponentAddress, Hash32},
    Network,
};
use serde_json::{json, Value};

const DEFAULT_WALLETD_URL: &str = "http://127.0.0.1:5100/json_rpc";
// 0.05 TARI. the wallet only charges what the transaction actually uses.
const DEFAULT_MAX_FEE: u64 = 50_000;

fn usage() -> ! {
    eprintln!(
        "tacita: private campaign payouts on tari ootle

usage:
  tacita doctor
      check the connection to your ootle wallet
  tacita check --winners winners.csv
      read a winners file and print its total, headcount and manifest root (offline)
  tacita create --template <template address> --name \"<round name>\" --winners winners.csv
      lock the round's total in a new campaign on chain
  tacita pay --campaign <component address> --winners winners.csv
      pay every winner from the campaign in one transaction, amounts hidden
  tacita close --campaign <component address>
      return anything left in the campaign to your account
  tacita fund --amount <TARI>
      move hidden TARI into your account's revealed balance (campaigns and fees use revealed funds)

options:
  --account <component address>   which account pays and owns (default: your default account)
  --max-fee <micro TARI>          fee cap (default {DEFAULT_MAX_FEE})
  --dry-run                       print the transaction instead of sending it

winners file: one `address,amount` per line, amounts in TARI, `#` for comments, at most 16 lines.

settings (environment variables):
  TACITA_API_KEY    api key from the wallet's 'API Keys' page (required to send)
  TACITA_WALLETD    wallet json-rpc url (default {DEFAULT_WALLETD_URL})"
    );
    std::process::exit(2)
}

struct Opts {
    cmd: String,
    flags: HashMap<String, String>,
    dry_run: bool,
}

impl Opts {
    fn parse() -> Self {
        let mut args = env::args().skip(1);
        let cmd = args.next().unwrap_or_default();
        let mut flags = HashMap::new();
        let mut dry_run = false;
        while let Some(a) = args.next() {
            match a.as_str() {
                "--dry-run" => dry_run = true,
                "-h" | "--help" => usage(),
                f if f.starts_with("--") => {
                    let v = args.next().unwrap_or_else(|| {
                        eprintln!("{f} needs a value");
                        std::process::exit(2)
                    });
                    flags.insert(f.trim_start_matches("--").to_string(), v);
                },
                other => {
                    eprintln!("unexpected argument `{other}`");
                    std::process::exit(2)
                },
            }
        }
        Self { cmd, flags, dry_run }
    }

    fn need(&self, name: &str) -> Result<&str, String> {
        self.flags
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("missing --{name}. run `tacita --help` for usage"))
    }

    fn max_fee(&self) -> Result<u64, String> {
        match self.flags.get("max-fee") {
            Some(v) => v.parse().map_err(|_| "--max-fee must be a whole number of micro TARI".into()),
            None => Ok(DEFAULT_MAX_FEE),
        }
    }

    fn winners(&self) -> Result<Manifest, String> {
        let path = self.need("winners")?;
        let text = fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
        Manifest::parse(&text)
    }
}

fn walletd() -> walletd::Walletd {
    let url = env::var("TACITA_WALLETD").unwrap_or_else(|_| DEFAULT_WALLETD_URL.to_string());
    let key = env::var("TACITA_API_KEY").unwrap_or_else(|_| {
        eprintln!("TACITA_API_KEY is not set. create a key on the wallet's 'API Keys' page first.");
        std::process::exit(2)
    });
    walletd::Walletd::new(url, key)
}

fn parse_component(s: &str) -> Result<ComponentAddress, String> {
    ComponentAddress::from_str(s.trim()).map_err(|e| format!("`{s}` is not a component address: {e:?}"))
}

fn parse_template(s: &str) -> Result<Hash32, String> {
    let s = s.trim();
    Hash32::from_str(s.strip_prefix("template_").unwrap_or(s)).map_err(|e| format!("`{s}` is not a template address: {e:?}"))
}

/// the account that pays fees and owns the campaign.
fn account(opts: &Opts, w: Option<&walletd::Walletd>) -> Result<ComponentAddress, String> {
    if let Some(a) = opts.flags.get("account") {
        return parse_component(a);
    }
    let w = w.ok_or("pass --account with --dry-run, or drop --dry-run so tacita can ask the wallet")?;
    let list = w.call("accounts.list", json!({ "offset": 0, "limit": 50 }))?;
    let accounts = list["accounts"].as_array().cloned().unwrap_or_default();
    let pick = accounts
        .iter()
        .find(|a| a["is_default"].as_bool() == Some(true))
        .or_else(|| accounts.first())
        .ok_or("the wallet has no accounts yet")?;
    let addr = pick["account"]["component_address"]
        .as_str()
        .ok_or("could not read the account address from the wallet")?;
    parse_component(addr)
}

fn submit_request(account: ComponentAddress, instructions: Value, max_fee: u64) -> Value {
    json!({
        "instructions": instructions,
        "fee_account": account.to_string(),
        "max_fee": max_fee,
    })
}

fn submit(w: &walletd::Walletd, account: ComponentAddress, instructions: Value, max_fee: u64) -> Result<Value, String> {
    let sent = w.call("transactions.submit_instruction", submit_request(account, instructions, max_fee))?;
    wait(w, sent)
}

fn wait(w: &walletd::Walletd, sent: Value) -> Result<Value, String> {
    let id = sent["transaction_id"].clone();
    println!("sent. transaction id: {}", id.as_str().unwrap_or(&id.to_string()));
    println!("waiting for the network (up to 2 minutes)...");
    let done = w.call("transactions.wait_result", json!({ "transaction_id": id, "timeout_secs": 120 }))?;
    if done["timed_out"].as_bool() == Some(true) {
        return Err("no result after 2 minutes. check the wallet's transactions page".into());
    }
    let status = done["status"].as_str().unwrap_or("unknown").to_string();
    println!("status: {status}, fee paid: {} TARI", fmt_tari(done["final_fee"].as_u64().unwrap_or(0)));
    if status != "Accepted" {
        let reason = find_reject_reason(&done).unwrap_or_else(|| done.to_string());
        return Err(format!("the network did not accept it: {reason}"));
    }
    Ok(done)
}

fn find_reject_reason(v: &Value) -> Option<String> {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                if k.to_lowercase().contains("reject") || k == "reason" {
                    return Some(val.to_string());
                }
                if let Some(r) = find_reject_reason(val) {
                    return Some(r);
                }
            }
            None
        },
        Value::Array(a) => a.iter().find_map(find_reject_reason),
        _ => None,
    }
}

/// every `component_...` string in a result, in order, without duplicates.
fn components_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.starts_with("component_") && !out.contains(s) => out.push(s.clone()),
        Value::Object(m) => m.values().for_each(|x| components_in(x, out)),
        Value::Array(a) => a.iter().for_each(|x| components_in(x, out)),
        _ => {},
    }
}

fn print_round(m: &Manifest) {
    println!("winners:       {}", m.headcount());
    println!("total:         {} TARI", fmt_tari(m.total()));
    println!("manifest root: {}", hex(&m.root()));
}

fn doctor() -> Result<(), String> {
    let w = walletd();
    let info = w.call("wallet.get_info", json!({}))?;
    println!("wallet:   {}", info);
    let accounts = w.call("accounts.list", json!({ "offset": 0, "limit": 20 }))?;
    println!("accounts: {}", accounts);
    let balances = w.call("accounts.get_balances", json!({ "account": null, "refresh": true }))?;
    println!("balances: {}", balances);
    println!("\nok: tacita can talk to your wallet.");
    Ok(())
}

fn check(opts: &Opts) -> Result<(), String> {
    let m = opts.winners()?;
    print_round(&m);
    println!("\nok: the winners file is valid.");
    Ok(())
}

fn create(opts: &Opts) -> Result<(), String> {
    let m = opts.winners()?;
    let name = opts.need("name")?;
    if name.len() > 80 {
        return Err("--name must be 80 characters or fewer".into());
    }
    let template = parse_template(opts.need("template")?)?;
    print_round(&m);

    let w = (!opts.dry_run).then(walletd);
    let account = account(opts, w.as_ref())?;
    let root = Hash32::from_array(m.root());
    let instructions = tx::create(account, template, name, Amount::from(m.total()), m.headcount(), root)?;

    let Some(w) = w else {
        println!("{}", serde_json::to_string_pretty(&instructions).unwrap());
        return Ok(());
    };
    let done = submit(&w, account, instructions, opts.max_fee()?)?;
    let mut found = Vec::new();
    components_in(&done, &mut found);
    let account_str = account.to_string();
    found.retain(|c| *c != account_str);
    match found.as_slice() {
        [] => println!("created, but tacita could not spot the campaign address. look for the new component in the wallet."),
        [one] => println!("\ncampaign: {one}\nkeep this. you need it for `tacita pay`."),
        many => println!("\ncreated. the campaign is one of these components:\n  {}", many.join("\n  ")),
    }
    Ok(())
}

fn pay(opts: &Opts) -> Result<(), String> {
    let m = opts.winners()?;
    let campaign = parse_component(opts.need("campaign")?)?;
    print_round(&m);

    // the statement has no stealth inputs (the campaign vault supplies a revealed bucket), so
    // the key that builds it never owns anything. a fresh one per payout leaks nothing.
    let builder = PrivateKeyProvider::random(Network::MainNet);
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let statement = rt
        .block_on(payout::build_statement(&builder, &m.lines))
        .map_err(|e| format!("could not build the hidden payout: {e:?}"))?;
    payout::check_statement(&statement, m.lines.len(), m.total())?;
    println!("hidden payout built and checked: {} outputs, nothing revealed.", m.lines.len());

    let instructions = tx::pay(campaign, statement)?;
    if opts.dry_run {
        println!("{}", serde_json::to_string_pretty(&instructions).unwrap());
        return Ok(());
    }
    let w = walletd();
    let account = account(opts, Some(&w))?;
    submit(&w, account, instructions, opts.max_fee()?)?;
    println!("\npaid. each winner's wallet will show the TARI once it scans the new outputs.");
    Ok(())
}

fn fund(opts: &Opts) -> Result<(), String> {
    let micro = manifest::parse_tari(opts.need("amount")?)?;
    if micro == 0 {
        return Err("--amount must be above 0".into());
    }
    let w = walletd();
    let list = w.call("accounts.list", json!({ "offset": 0, "limit": 50 }))?;
    let accounts = list["accounts"].as_array().cloned().unwrap_or_default();
    let pick = match opts.flags.get("account") {
        Some(a) => accounts.iter().find(|x| x["account"]["component_address"].as_str() == Some(a.as_str())),
        None => accounts.iter().find(|x| x["account"]["is_default"].as_bool() == Some(true)).or_else(|| accounts.first()),
    }
    .ok_or("could not find that account in the wallet")?;
    let account = pick["account"]["component_address"].as_str().ok_or("account has no address")?;
    let own = pick["address"].as_str().ok_or("could not read the account's ootle address")?;
    println!("moving {} TARI from hidden to revealed in {account}", fmt_tari(micro));
    let req = fund::request(account, own, micro, opts.max_fee()?);
    if opts.dry_run {
        println!("{}", serde_json::to_string_pretty(&req).unwrap());
        return Ok(());
    }
    let sent = w.call("accounts.stealth_transfer", req)?;
    wait(&w, sent)?;
    println!("\ndone. run `tacita doctor` to see the new revealed balance.");
    Ok(())
}

fn close(opts: &Opts) -> Result<(), String> {
    let campaign = parse_component(opts.need("campaign")?)?;
    let w = (!opts.dry_run).then(walletd);
    let account = account(opts, w.as_ref())?;
    let instructions = tx::close(campaign, account)?;
    let Some(w) = w else {
        println!("{}", serde_json::to_string_pretty(&instructions).unwrap());
        return Ok(());
    };
    submit(&w, account, instructions, opts.max_fee()?)?;
    println!("\nclosed. anything left is back in your account.");
    Ok(())
}

fn main() {
    let opts = Opts::parse();
    let result = match opts.cmd.as_str() {
        "doctor" => doctor(),
        "check" => check(&opts),
        "create" => create(&opts),
        "pay" => pay(&opts),
        "close" => close(&opts),
        "fund" => fund(&opts),
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ootle_rs::{key_provider::PrivateKeyProvider, transaction::TransactionSigner};
    use tari_ootle_walletd_client::types::CallInstructionRequest;

    use super::*;

    const ACCOUNT: &str = "component_5286446acff808c0fcad494ce7b74e19238b446b02394242328d974261d3fe36";
    const TEMPLATE: &str = "template_7d80993c271e39f70577f4a9f63c667d415cc64cafba4465d3b96d6607a5bc46";

    fn round_trips(v: Value) {
        let parsed: CallInstructionRequest = serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}\n{v:#}"));
        assert!(!parsed.instructions.is_empty());
    }

    #[test]
    fn the_wallet_accepts_create_close_and_pay() {
        let account = ComponentAddress::from_str(ACCOUNT).unwrap();
        let template = parse_template(TEMPLATE).unwrap();
        round_trips(submit_request(account, tx::create(account, template, "round 1", Amount::from(1_500_000u64), 3, Hash32::zero()).unwrap(), DEFAULT_MAX_FEE));
        round_trips(submit_request(account, tx::close(account, account).unwrap(), DEFAULT_MAX_FEE));

        let lines: Vec<payout::Line> = (0..3)
            .map(|i| payout::Line {
                address: PrivateKeyProvider::random(Network::MainNet).address().clone(),
                amount: std::num::NonZeroU64::new(500_000 + i).unwrap(),
            })
            .collect();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let st = rt.block_on(payout::build_statement(&PrivateKeyProvider::random(Network::MainNet), &lines)).unwrap();
        round_trips(submit_request(account, tx::pay(account, st).unwrap(), DEFAULT_MAX_FEE));
    }
}
