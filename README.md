# hush

pay people privately, and still prove every token.

hush is a tari ootle template for campaign and treasury payouts. each winner's amount stays hidden, the contract enforces that the full announced total goes to exactly the announced number of people, each winner can prove their own line, and an auditor file lets a reviewer check every line against the chain.

> work in progress for the tari november mvp build contest.

## the rules `pay` enforces

1. only the campaign owner can pay (checked through the transaction signer)
2. a round can only be paid once
3. exactly one hidden output per winner (max 16 per round, ootle's stealth output limit)
4. no output carries a viewable balance proof
5. no output promises a minimum value
6. nothing is paid out in the clear
7. the amount taken from the vault equals the announced round total

## build

```
rustup target add wasm32-unknown-unknown
cd template
cargo build --target wasm32-unknown-unknown --release
```

## license

mit
