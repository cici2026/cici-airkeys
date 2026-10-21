//! What the phone's work costs: opening a wallet (PBKDF2 + the hardened derivations), deriving
//! the addresses of an account, checking and signing requests. Run it on a phone:
//!   cargo ndk -t arm64-v8a build --release -p airkeys-wallet --example bench
//!   adb push target/aarch64-linux-android/release/examples/bench /data/local/tmp/ && adb shell /data/local/tmp/bench
use airkeys_wallet::protocol::Message;
use airkeys_wallet::{Mnemonic, Network, Wallet, review};
use std::time::Instant;

fn time<T>(what: &str, n: u32, mut f: impl FnMut() -> T) {
    f();
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(f());
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
    println!("{what:<48} {:>10.1} µs", us);
}

fn main() {
    let phrase = Mnemonic::parse(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    time("open a wallet (PBKDF2-SHA512 ×2048, 2 parents)", 20, || {
        Wallet::new(&phrase.seed("").unwrap()).unwrap()
    });
    let w = Wallet::new(&phrase.seed("").unwrap()).unwrap();
    time("the 2 addresses of an account", 200, || Network::ALL.map(|n| w.address(n, 7).unwrap()));
    time("all 200 addresses of a full connect (#0-#99, both networks)", 10, || {
        (0..100).flat_map(|i| Network::ALL.map(|n| w.address(n, i).unwrap())).count()
    });
    let from = w.address(Network::Evm, 0).unwrap();
    let txs: Vec<String> = (0..16)
        .map(|k| format!(r#"{{"to":{{"index":1}},"amount":"1000","token":{{"address":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","symbol":"USDC","decimals":6}},"nonce":{k},"gas":90000,"maxFeePerGas":"30000000000","maxPriorityFeePerGas":"1000000000"}}"#))
        .collect();
    let json = format!(
        r#"{{"airkeys":1,"type":"sign","id":"b","network":"evm","chainId":1,"from":{{"index":0,"address":"{from}"}},"txs":[{}]}}"#,
        txs.join(",")
    );
    let Message::Sign(req) = Message::parse(json.as_bytes()).unwrap() else { panic!() };
    time("check an EVM request of 16 transactions", 200, || review(&req, &w).unwrap());
    let r = review(&req, &w).unwrap();
    time("sign it (16 ECDSA signatures)", 200, || r.sign(&w).unwrap());
}
