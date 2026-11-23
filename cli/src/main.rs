mod payout;
mod walletd;

use std::env;

use serde_json::json;

const DEFAULT_WALLETD_URL: &str = "http://127.0.0.1:5100/json_rpc";

fn usage() -> ! {
    eprintln!(
        "hush: private campaign payouts on tari ootle\n\n\
         usage:\n  hush doctor     check the connection to your ootle wallet\n\n\
         settings (environment variables):\n  HUSH_API_KEY    api key from the wallet's 'API Keys' page (required)\n  \
         HUSH_WALLETD    wallet json-rpc url (default {DEFAULT_WALLETD_URL})"
    );
    std::process::exit(2)
}

fn walletd() -> walletd::Walletd {
    let url = env::var("HUSH_WALLETD").unwrap_or_else(|_| DEFAULT_WALLETD_URL.to_string());
    let key = env::var("HUSH_API_KEY").unwrap_or_else(|_| {
        eprintln!("HUSH_API_KEY is not set. create a key on the wallet's 'API Keys' page first.");
        std::process::exit(2)
    });
    walletd::Walletd::new(url, key)
}

fn doctor() -> Result<(), String> {
    let w = walletd();
    let info = w.call("wallet.get_info", json!({}))?;
    println!("wallet:   {}", info);
    let accounts = w.call("accounts.list", json!({ "offset": 0, "limit": 20 }))?;
    println!("accounts: {}", accounts);
    let balances = w.call("accounts.get_balances", json!({ "account": null, "refresh": true }))?;
    println!("balances: {}", balances);
    println!("\nok: hush can talk to your wallet.");
    Ok(())
}

fn main() {
    let cmd = env::args().nth(1).unwrap_or_default();
    let result = match cmd.as_str() {
        "doctor" => doctor(),
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
