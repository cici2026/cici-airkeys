//! The phone's signing path as a command, for tests and for checking requests by hand.
//!
//! ```sh
//! AIRKEYS_PHRASE="abandon … about" cargo run -p airkeys-wallet --example sign < request.json  # review + signed
//! AIRKEYS_PHRASE="…" cargo run -p airkeys-wallet --example sign -- accounts evm 0 1            # an accounts message
//! ```
//!
//! Prints the review (what the phone shows) on stderr and the reply message on stdout.
//!
//! Test phrases only: a phrase in an environment variable lands in shell history and process
//! listings. Never use one that holds funds.

use airkeys_wallet::protocol::{Account, Message};
use airkeys_wallet::{Mnemonic, Network, Wallet, review};
use std::io::Read;

fn main() {
    let Ok(phrase) = std::env::var("AIRKEYS_PHRASE") else {
        eprintln!(
            "usage: AIRKEYS_PHRASE=\"<test phrase>\" sign < request.json | sign accounts <network> <index>…"
        );
        std::process::exit(2);
    };
    let passphrase = std::env::var("AIRKEYS_PASSPHRASE").unwrap_or_default();
    let wallet = Wallet::new(&Mnemonic::parse(&phrase).unwrap().seed(&passphrase).unwrap()).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("accounts") {
        let network: Network =
            serde_json::from_value(serde_json::Value::String(args[1].clone())).expect("a network");
        let accounts = args[2..]
            .iter()
            .map(|i| {
                let index = i.parse().unwrap();
                Account { network, index, address: wallet.address(network, index).unwrap() }
            })
            .collect();
        println!("{}", Message::Accounts(accounts).to_json());
        return;
    }
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    let request = match Message::parse(&input) {
        Ok(Message::Sign(r)) => r,
        Ok(m) => fail(&format!("expected a sign request, got “{}”", m.type_name())),
        Err(e) => fail(&e.0),
    };
    let review = review(&request, &wallet).unwrap_or_else(|e| fail(&e.0));
    eprintln!("{}", serde_json::to_string_pretty(&review).unwrap());
    let signed = review.sign(&wallet).unwrap_or_else(|e| fail(&e.0));
    println!("{}", Message::Signed(signed).to_json());
}

fn fail(msg: &str) -> ! {
    eprintln!("refused: {msg}");
    std::process::exit(2)
}
