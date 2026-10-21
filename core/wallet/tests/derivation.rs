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
        (0, "bc1q8d2x9494zdyx6ka8pp4p94xe8jpsef77pzl7td", "0xcf6335E1C68B1F7C9D8Aa84c647FFcd9595f0b03"),
        (1, "bc1qaqw9zun4ysc4etc7pg36xfxgwzppcl6m0vxatg", "0x4B4dE798e77cF830AdCb49Cccb3D75bf2A149327"),
        (7, "bc1qtl06yprx0gz3daqqqmgrg2mftdpd6q0furkts9", "0x99b93685439306C171F167a876BDC3880ffF7cbf"),
    ];
    for (i, btc, evm) in expect {
        assert_eq!(w.address(Network::Bitcoin, i).unwrap(), btc);
        assert_eq!(w.address(Network::Evm, i).unwrap(), evm);
    }
}

#[test]
fn passphrase_gives_another_wallet() {
    let w = wallet("TREZOR");
    assert_eq!(w.address(Network::Bitcoin, 0).unwrap(), "bc1qf24jdufxvfc9metp6kp6y7q7hnx7xs6zu54es3");
    assert_eq!(w.address(Network::Evm, 0).unwrap(), "0xeD9fEfdE5cd2905De816068735350Cf3b111e46a");
}

#[test]
fn paths_are_fully_hardened() {
    assert_eq!(Network::Bitcoin.path(3), "m/84'/0'/0'/0'/3'");
    assert_eq!(Network::Evm.path(3), "m/44'/60'/0'/0'/3'");
    for n in Network::ALL {
        assert!(n.path(0).split('/').skip(1).all(|s| s.ends_with('\'')));
    }
}
