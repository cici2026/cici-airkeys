//! Values the web app's own code must agree on, printed as JSON for its unit tests
//! (app-web/airkeys.app/src/lib/vectors.json): addresses parsed.
use airkeys_wallet::{bitcoin, evm};

fn main() {
    let btc: Vec<_> = ["bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3", "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0", "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2", "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy", "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t5", "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx", "bc1zw508d6qejxtdg4y5r3zarvaryvaxxpcs"]
        .iter()
        .map(|a| serde_json::json!({ "address": a, "script": bitcoin::parse_address(a).ok().map(|p| p.script.iter().map(|b| format!("{b:02x}")).collect::<String>()) }))
        .collect();
    let evm: Vec<_> = ["0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed", "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAeD", "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeA"]
        .iter()
        .map(|a| serde_json::json!({ "address": a, "checksum": evm::parse_address(a).ok().map(|x| evm::checksum(&x)) }))
        .collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "bitcoin": btc, "evm": evm })).unwrap());
}
