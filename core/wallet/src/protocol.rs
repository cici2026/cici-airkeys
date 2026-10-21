//! The Airkeys wire protocol, version 1: the three messages and their exact JSON
//! ([`docs/PROTOCOL.md`](../../../docs/PROTOCOL.md)). What a request means and whether the
//! phone signs it is [`crate::review`].

use crate::amount::Amount;
use crate::{Network, Result, bail, shown};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const VERSION: u64 = 1;
/// The largest message a receiver accepts (PROTOCOL §2.6).
pub const MAX_MESSAGE: usize = 1 << 20;
/// Every account number the phone offers (100) on both networks.
pub const MAX_ACCOUNTS: usize = 200;

/// An account of this wallet, as `accounts` and `signed` list it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub network: Network,
    pub index: u32,
    pub address: String,
}

/// The account a request spends from: both fields, so the phone can tell that the request
/// was made for this wallet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub index: u32,
    pub address: String,
}

/// An account reference (PROTOCOL §3): an address, an account of this wallet, or both.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenRef {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

// ---------------------------------------------------------------- sign requests

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BtcInput {
    pub txid: String,
    pub vout: u32,
    pub amount: Amount,
    pub prev_tx: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BtcOutput {
    pub to: Target,
    pub amount: Amount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BitcoinSign {
    pub inputs: Vec<BtcInput>,
    pub outputs: Vec<BtcOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvmTx {
    pub to: Target,
    pub amount: Amount,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<TokenRef>,
    pub nonce: u64,
    pub gas: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<Amount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<Amount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<Amount>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvmSign {
    pub chain_id: u64,
    pub txs: Vec<EvmTx>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    Bitcoin(BitcoinSign),
    Evm(EvmSign),
}

impl Body {
    pub fn network(&self) -> Network {
        match self {
            Body::Bitcoin(_) => Network::Bitcoin,
            Body::Evm(_) => Network::Evm,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    pub id: String,
    pub from: Source,
    pub body: Body,
}

// ---------------------------------------------------------------- signed

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTx {
    pub hash: String,
    pub raw: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed {
    pub id: String,
    pub network: Network,
    pub txs: Vec<SignedTx>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Account>,
}

// ---------------------------------------------------------------- messages

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Accounts(Vec<Account>),
    Sign(SignRequest),
    Signed(Signed),
}

/// `id`: 1–64 characters of `[A-Za-z0-9_-]` (PROTOCOL §5).
pub fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn take<T: DeserializeOwned>(map: &mut Map<String, Value>, key: &str) -> Result<T> {
    let Some(v) = map.remove(key) else { bail!("The message has no “{key}”.") };
    serde_json::from_value(v)
        .map_err(|e| crate::Error(format!("“{key}” is not valid: {}.", shown(&e.to_string()))))
}

fn rest<T: DeserializeOwned>(map: Map<String, Value>) -> Result<T> {
    serde_json::from_value(Value::Object(map))
        .map_err(|e| crate::Error(format!("The message is not valid: {}.", shown(&e.to_string()))))
}

impl Message {
    /// Parses and checks the shape of a message (§2). Whether a request can be signed is
    /// decided by [`crate::review::review`].
    pub fn parse(bytes: &[u8]) -> Result<Message> {
        if bytes.len() > MAX_MESSAGE {
            bail!("The message is larger than 1 MiB.");
        }
        let Ok(Value::Object(mut map)) = serde_json::from_slice::<Value>(bytes) else {
            bail!("This is not an Airkeys message.");
        };
        match map.remove("airkeys") {
            Some(Value::Number(n)) if n.as_u64() == Some(VERSION) => {}
            Some(Value::Number(n)) => {
                bail!("This message uses Airkeys protocol version {n}. This app speaks version {VERSION}.")
            }
            _ => bail!("This is not an Airkeys message."),
        }
        let kind: String = take(&mut map, "type")?;
        let message = match kind.as_str() {
            "accounts" => {
                let accounts: Vec<Account> = take(&mut map, "accounts")?;
                if !(1..=MAX_ACCOUNTS).contains(&accounts.len()) {
                    bail!("An accounts message lists 1 to {MAX_ACCOUNTS} accounts.");
                }
                if let Some(k) = map.keys().next() {
                    bail!("The message has an unknown field “{k}”.");
                }
                Message::Accounts(accounts)
            }
            "sign" => {
                let id: String = take(&mut map, "id")?;
                if !valid_id(&id) {
                    bail!("The request id is not valid.");
                }
                let network: Network = take(&mut map, "network")?;
                let from: Source = take(&mut map, "from")?;
                let body = match network {
                    Network::Bitcoin => Body::Bitcoin(rest(map)?),
                    Network::Evm => Body::Evm(rest(map)?),
                };
                Message::Sign(SignRequest { id, from, body })
            }
            "signed" => Message::Signed(rest(map)?),
            other => bail!("“{other}” is not an Airkeys message type."),
        };
        Ok(message)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Message::Accounts(_) => "accounts",
            Message::Sign(_) => "sign",
            Message::Signed(_) => "signed",
        }
    }

    /// Compact JSON: `airkeys` and `type` first, then the fields in the order of §4–§6.
    pub fn to_json(&self) -> String {
        #[derive(Serialize)]
        struct Envelope<'a, T: Serialize> {
            airkeys: u64,
            #[serde(rename = "type")]
            kind: &'a str,
            #[serde(flatten)]
            body: T,
        }
        #[derive(Serialize)]
        struct AccountsBody<'a> {
            accounts: &'a [Account],
        }
        #[derive(Serialize)]
        struct SignBody<'a, B: Serialize> {
            id: &'a str,
            network: Network,
            from: &'a Source,
            #[serde(flatten)]
            body: &'a B,
        }
        fn json<T: Serialize>(kind: &str, body: T) -> String {
            serde_json::to_string(&Envelope { airkeys: VERSION, kind, body }).expect("serialisable")
        }
        let kind = self.type_name();
        match self {
            Message::Accounts(accounts) => json(kind, AccountsBody { accounts }),
            Message::Signed(signed) => json(kind, signed),
            Message::Sign(r) => {
                let (id, network, from) = (r.id.as_str(), r.body.network(), &r.from);
                match &r.body {
                    Body::Bitcoin(body) => json(kind, SignBody { id, network, from, body }),
                    Body::Evm(body) => json(kind, SignBody { id, network, from, body }),
                }
            }
        }
    }

    /// The Lumen Code file name (PROTOCOL §2.1).
    pub fn file_name(&self) -> String {
        format!("airkeys-{}.json", self.type_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_rules() {
        let e = |s: &str| Message::parse(s.as_bytes()).err().unwrap().0;
        assert!(e("[]").contains("not an Airkeys"));
        assert!(e(r#"{"type":"accounts"}"#).contains("not an Airkeys"));
        assert!(e(r#"{"airkeys":2,"type":"accounts"}"#).contains("version 2"));
        assert!(e(r#"{"airkeys":"1","type":"accounts"}"#).contains("not an Airkeys"));
        assert!(e(r#"{"airkeys":1,"type":"hello"}"#).contains("not an Airkeys message type"));
        assert!(e(r#"{"airkeys":1,"type":"accounts","accounts":[]}"#).contains("1 to 200"));
        let ok =
            r#"{"airkeys":1,"type":"accounts","accounts":[{"network":"evm","index":0,"address":"0x00"}]}"#;
        assert!(Message::parse(ok.as_bytes()).is_ok());
        let extra = ok.replace("}]}", "}],\"x\":1}");
        assert!(Message::parse(extra.as_bytes()).is_err());
        let extra = ok.replace("\"0x00\"}", "\"0x00\",\"path\":\"m\"}");
        assert!(Message::parse(extra.as_bytes()).is_err());
    }

    #[test]
    fn round_trips() {
        let json = r#"{"airkeys":1,"type":"sign","id":"a-1","network":"bitcoin","from":{"index":0,"address":"bc1q"},"inputs":[{"txid":"ab","vout":1,"amount":"5","prevTx":"00"}],"outputs":[{"to":{"index":1},"amount":"4"}]}"#;
        let m = Message::parse(json.as_bytes()).unwrap();
        assert_eq!(m.to_json(), json);
        let Message::Sign(r) = &m else { panic!() };
        assert_eq!(r.body.network(), Network::Bitcoin);
        let signed = Message::Signed(Signed {
            id: "x".into(),
            network: Network::Evm,
            txs: vec![SignedTx { hash: "0x1".into(), raw: "0x2".into() }],
            accounts: vec![],
        });
        assert_eq!(
            signed.to_json(),
            r#"{"airkeys":1,"type":"signed","id":"x","network":"evm","txs":[{"hash":"0x1","raw":"0x2"}]}"#
        );
        assert_eq!(Message::parse(signed.to_json().as_bytes()).unwrap(), signed);
    }

    #[test]
    fn ids() {
        assert!(valid_id("c0ffee-01_A"));
        assert!(!valid_id(""));
        assert!(!valid_id("a b"));
        assert!(!valid_id(&"a".repeat(65)));
    }
}
