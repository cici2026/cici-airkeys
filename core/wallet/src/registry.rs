//! The networks and tokens Airkeys knows by name: [`registry/networks.json`], shared with the
//! web app. The phone uses names, native coins and the token list; RPC and explorer URLs are
//! for the web app and ignored here.
//!
//! [`registry/networks.json`]: ../../../registry/networks.json

use crate::Network;
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chain {
    /// EVM chains only.
    pub chain_id: Option<u64>,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(default)]
    pub tokens: Vec<Token>,
    /// A test network: its coins have no value.
    #[serde(default)]
    pub testnet: bool,
}

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub bitcoin: Chain,
    pub evm: Vec<Chain>,
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../registry/networks.json")).expect("registry/networks.json")
});

pub fn get() -> &'static Registry {
    &REGISTRY
}

impl Registry {
    pub fn evm_chain(&self, chain_id: u64) -> Option<&Chain> {
        self.evm.iter().find(|c| c.chain_id == Some(chain_id))
    }

    /// The chain of a non-EVM network.
    pub fn chain(&self, network: Network) -> Option<&Chain> {
        match network {
            Network::Bitcoin => Some(&self.bitcoin),
            Network::Evm => None,
        }
    }
}

impl Chain {
    /// A listed token, its address compared without regard to case (EIP-55 is a checksum).
    pub fn token(&self, address: &str) -> Option<&Token> {
        self.tokens.iter().find(|t| t.address.eq_ignore_ascii_case(address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_consistent() {
        let r = get();
        assert_eq!(r.evm_chain(1).unwrap().name, "Ethereum");
        assert!(
            !r.evm_chain(1).unwrap().testnet && r.evm_chain(11_155_111).unwrap().testnet,
            "Sepolia is a testnet"
        );
        let mut ids: Vec<_> = r.evm.iter().map(|c| c.chain_id.unwrap()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), r.evm.len(), "chain ids are unique");
        for c in &r.evm {
            for t in &c.tokens {
                let a = crate::evm::parse_address(&t.address).unwrap();
                assert_eq!(crate::evm::checksum(&a), t.address, "{} {} is checksummed", c.name, t.symbol);
            }
        }
        assert!(r.evm_chain(1).unwrap().token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").is_some());
    }
}
