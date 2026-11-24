# tacita

pay people privately, and still prove every token.

tacita is a tari ootle template for campaign and treasury payouts. each winner's amount stays hidden, the contract enforces that the full announced total goes to exactly the announced number of people, each winner can prove their own line, and an auditor file lets a reviewer check every line against the chain.

> named after tacita, the roman goddess of silence. the amounts are never said, the total is always proven.
>
> work in progress for the tari november mvp build contest.

## on mainnet

| what | value |
|---|---|
| template (`Campaign`) | `template_7d80993c271e39f70577f4a9f63c667d415cc64cafba4465d3b96d6607a5bc46` |
| publish transaction | `ddae0f5144abfd1b31fc3e30668493bde513cfd0832bf60c15870d8e5170c69f` (accepted, 24 nov 2026, fee 2.891809 TARI) |

## the rules `pay` enforces

1. only the campaign owner can pay (checked through the transaction signer)
2. a round can only be paid once
3. exactly one hidden output per winner (max 16 per round, ootle's stealth output limit)
4. no output carries a viewable balance proof
5. no output promises a minimum value
6. nothing is paid out in the clear
7. the amount taken from the vault equals the announced round total

## use

tacita talks to your own ootle wallet (walletd). the wallet signs and pays the fee; tacita never sees your keys.

1. in the wallet, open **API Keys** and create a key. in a terminal: `set TACITA_API_KEY=tw_...` (windows) or `export TACITA_API_KEY=tw_...`
2. `tacita doctor` checks the connection.
3. write a winners file, one `address,amount` per line (amounts in TARI, at most 16 lines):
   ```
   # round 1
   otl_1abc...,1.5
   otl_1def...,0.75
   ```
4. `tacita check --winners winners.csv` prints the total, headcount and manifest root.
5. `tacita create --template <template address> --name "round 1" --winners winners.csv` locks the total in a new campaign.
6. `tacita pay --campaign <component address> --winners winners.csv` pays everyone in one transaction, amounts hidden.
7. `tacita close --campaign <component address>` returns anything left.

add `--dry-run` to any command to see the transaction without sending it.

## build

```
rustup target add wasm32-unknown-unknown
cd template
cargo build --target wasm32-unknown-unknown --release
```

## license

mit
