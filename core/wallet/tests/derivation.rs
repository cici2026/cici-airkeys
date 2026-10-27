//! Addresses on the Airkeys paths (PROTOCOL §3), checked against independent
//! implementations: Foundry's `cast wallet address` for EVM, paulmillr's @scure/bip32 and
//! @scure/bip39 and Bitcoin Core's descriptors for Bitcoin.

use airkeys_wallet::{Mnemonic, Network, Wallet};

const PHRASE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn wallet(passphrase: &str) -> Wallet {
    Wallet::new(&Mnemonic::parse(PHRASE).unwrap().seed(passphrase).unwrap()).unwrap()
}

#[test]
fn addresses_match_independent_implementations() {
    let w = wallet("");
    let expect = [
        (0, "bc1qvuapkt7kdu4vnrfttvzmszuj8cfdnedsch0hy3", "0xF23521DE6e2B111B384c1308b3c59Acc7Bf5e158"),
        (1, "bc1q3w9xwgw767utv5gv3uz3dh0skq6ymhqhwjvz28", "0xEA373B5359e70C7fE76d048fdfe435f6b910Ccd8"),
        (7, "bc1q7367t0ard3dgwaasd2ws8nt75qjjgplrmqywcw", "0x1445Be1AACC22976Eb39F6957b5CA8492dD48e19"),
    ];
    for (i, btc, evm) in expect {
        assert_eq!(w.address(Network::Bitcoin, i).unwrap(), btc);
        assert_eq!(w.address(Network::Evm, i).unwrap(), evm);
    }
}

#[test]
fn passphrase_gives_another_wallet() {
    let w = wallet("TREZOR");
    assert_eq!(w.address(Network::Bitcoin, 0).unwrap(), "bc1qnyydjtsxyx0u2s7tj6ujrqvswvnpr5fhrpj4fw");
    assert_eq!(w.address(Network::Evm, 0).unwrap(), "0x1930d009d71be5d487211a83E51b5973b90ee69b");
}

#[test]
fn paths_are_fully_hardened() {
    assert_eq!(Network::Bitcoin.path(3), "m/65738275'/84'/0'/0'/0'/3'");
    assert_eq!(Network::Evm.path(3), "m/65738275'/44'/60'/0'/0'/3'");
    for n in Network::ALL {
        assert!(n.path(0).split('/').skip(1).all(|s| s.ends_with('\'')));
    }
}
