//! Whether the phone signs a request, what it shows first, and the signing itself
//! (PROTOCOL §5–§7).
//!
//! [`review`] resolves every account, parses every address, checks every rule and builds
//! the exact transactions — without signing. The person sees the [`Review`]; only when they
//! confirm does [`Review::sign`] run. Nothing between the two can change what is signed.

use crate::amount::{Amount, format_u128};
use crate::hash::{hex, unhex};
use crate::protocol::*;
use crate::registry::{self, Chain};
use crate::{MAX_INDEX, Network, Result, Wallet, bail, bitcoin, evm, shown};
use serde::Serialize;

/// The most transactions an EVM request may hold (PROTOCOL §5.2).
pub const MAX_TXS: usize = 16;
/// The most coins a Bitcoin request may spend (PROTOCOL §5.1).
pub const MAX_INPUTS: usize = 200;
/// The most payments a Bitcoin request may make (PROTOCOL §5.1).
pub const MAX_OUTPUTS: usize = 16;
/// A fee above this share of what is sent gets a warning (§7).
const HIGH_FEE_PERCENT: u128 = 5;
const HIGH_FEE_RATE: u64 = 1000;
/// A price per gas above this (1 000 gwei) gets a warning on any EVM chain.
const HIGH_GAS_PRICE: u128 = 1_000_000_000_000;

/// One transfer, ready to show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Transfer {
    pub to: String,
    /// Set when `to` is an account of this wallet.
    pub own: Option<u32>,
    /// In whole units, e.g. "1.5".
    pub amount: String,
    /// The native coin's or the token's symbol.
    pub asset: String,
    /// The token contract; `None` for the native coin.
    pub token: Option<String>,
    /// The token is in Airkeys' list (always true for the native coin).
    pub verified: bool,
    /// The most this transaction can cost in fees (EVM only; a Bitcoin fee is the whole
    /// request's, in [`Review::fee`]).
    pub fee: Option<String>,
    pub note: Option<String>,
}

/// A request the phone can sign, as the person sees it before deciding.
#[derive(Debug, Serialize)]
pub struct Review {
    pub id: String,
    pub network: Network,
    /// "Bitcoin", "Ethereum", "Chain 12345".
    pub chain: String,
    /// The chain is a listed test network (Sepolia): its coins have no value.
    pub testnet: bool,
    pub from: Account,
    pub transfers: Vec<Transfer>,
    /// The most the whole request can cost in network fees.
    pub fee: String,
    /// Technical fields: label, value.
    pub details: Vec<(String, String)>,
    /// Something may be wrong; the person should look before signing.
    pub warnings: Vec<String>,
    /// Facts the person should know (what signing reveals).
    pub notices: Vec<String>,
    #[serde(skip)]
    plan: Plan,
    #[serde(skip)]
    new_accounts: Vec<Account>,
}

#[derive(Debug)]
enum Plan {
    Bitcoin { inputs: Vec<bitcoin::Input>, outputs: Vec<bitcoin::Output> },
    Evm(Vec<evm::Tx>),
}

/// What a request's own-account references resolve to.
struct Accounts<'a> {
    wallet: &'a Wallet,
    network: Network,
    from: u32,
    from_address: String,
    /// Accounts named by index alone (reported back in `signed`).
    new: Vec<Account>,
}

impl Accounts<'_> {
    /// Resolves a target to (address, own index); refuses the paying account itself.
    fn resolve(&mut self, t: &Target) -> Result<(String, Option<u32>)> {
        let (address, own) = match (t.index, &t.address) {
            (None, None) => bail!("A recipient has neither an address nor an account number."),
            (None, Some(a)) => (a.clone(), None),
            (Some(i), given) => {
                if i > MAX_INDEX {
                    bail!("Account numbers go up to {MAX_INDEX}.");
                }
                let derived = self.wallet.address(self.network, i)?;
                match given {
                    Some(a) if *a != derived => bail!(
                        "The request says account #{i} is {}, but this wallet's account #{i} is {derived}.",
                        shown(a)
                    ),
                    Some(_) => {}
                    None => {
                        let acc = Account { network: self.network, index: i, address: derived.clone() };
                        if !self.new.contains(&acc) {
                            self.new.push(acc);
                        }
                    }
                }
                (derived, Some(i))
            }
        };
        if own == Some(self.from) || self.pays_from(&address) {
            bail!(
                "A transfer goes back to the account that pays. That keeps funds at an address whose public key this transaction reveals."
            );
        }
        Ok((address, own))
    }
}

impl Accounts<'_> {
    /// Whether `address` pays the paying account, however it is written: EVM addresses in any
    /// case, Bitcoin addresses in any case and as the legacy or nested form of the same key.
    fn pays_from(&self, address: &str) -> bool {
        match self.network {
            Network::Evm => matches!(
                (evm::parse_address(address), evm::parse_address(&self.from_address)),
                (Ok(a), Ok(b)) if a == b
            ),
            Network::Bitcoin => {
                let (Ok(to), Ok(from)) =
                    (bitcoin::parse_address(address), bitcoin::parse_address(&self.from_address))
                else {
                    return false;
                };
                bitcoin::same_key_scripts(&from.script).contains(&to.script)
            }
        }
    }
}

fn count(n: usize, max: usize, what: &str) -> Result<()> {
    if !(1..=max).contains(&n) {
        bail!("A request has 1 to {max} {what}, not {n}.");
    }
    Ok(())
}

fn nonzero(a: &Amount) -> Result<()> {
    if a.is_zero() {
        bail!("Every amount must be more than zero.");
    }
    Ok(())
}

/// Checks a token against the list: `Ok(true)` listed and matching, `Ok(false)` unlisted.
fn check_token(chain: Option<&Chain>, t: &TokenRef) -> Result<bool> {
    let sym_ok = (1..=16).contains(&t.symbol.len()) && t.symbol.bytes().all(|b| b.is_ascii_graphic());
    if !sym_ok {
        bail!(
            "The token symbol “{}” is not allowed: use 1 to 16 printable ASCII characters.",
            shown(&t.symbol)
        );
    }
    if t.decimals > 36 {
        bail!("A token with {} decimals is not supported.", t.decimals);
    }
    let Some(listed) = chain.and_then(|c| c.token(&t.address)) else { return Ok(false) };
    if listed.symbol != t.symbol || listed.decimals != t.decimals {
        bail!(
            "The request calls the token at {} “{}” with {} decimals, but it is {} with {} decimals.",
            t.address,
            t.symbol,
            t.decimals,
            listed.symbol,
            listed.decimals
        );
    }
    Ok(true)
}

fn unlisted_warning(t: &TokenRef) -> String {
    format!(
        "{} at {} is not in Airkeys' token list. Its name and decimals come from the web app: check the contract address before you sign.",
        t.symbol, t.address
    )
}

fn high_fee(fee: u128, sent: u128) -> bool {
    sent > 0 && fee.saturating_mul(100) > sent.saturating_mul(HIGH_FEE_PERCENT)
}

fn exposure_notice(network: Network, index: u32) -> String {
    format!(
        "Broadcasting reveals the public key of {network} account #{index}. Funds that stay at this address are no longer hidden."
    )
}

/// Checks a request against this wallet and builds what it would sign (§7).
pub fn review(request: &SignRequest, wallet: &Wallet) -> Result<Review> {
    let network = request.body.network();
    let from = &request.from;
    if from.index > MAX_INDEX {
        bail!("Account numbers go up to {MAX_INDEX}.");
    }
    let derived = wallet.address(network, from.index)?;
    if derived != from.address {
        bail!(
            "This request is for another wallet: it spends from {}, but account #{} of this wallet is {derived}. Check the recovery phrase and passphrase.",
            shown(&from.address),
            from.index
        );
    }
    let mut accounts =
        Accounts { wallet, network, from: from.index, from_address: derived.clone(), new: Vec::new() };
    let mut r = Review {
        id: request.id.clone(),
        network,
        chain: String::new(),
        testnet: false,
        from: Account { network, index: from.index, address: derived },
        transfers: Vec::new(),
        fee: String::new(),
        details: Vec::new(),
        warnings: Vec::new(),
        notices: vec![exposure_notice(network, from.index)],
        plan: Plan::Evm(Vec::new()),
        new_accounts: Vec::new(),
    };
    match &request.body {
        Body::Bitcoin(b) => review_bitcoin(b, wallet, &mut accounts, &mut r)?,
        Body::Evm(b) => review_evm(b, &mut accounts, &mut r)?,
    }
    r.new_accounts = accounts.new;
    Ok(r)
}

fn review_bitcoin(b: &BitcoinSign, wallet: &Wallet, accounts: &mut Accounts, r: &mut Review) -> Result<()> {
    count(b.inputs.len(), MAX_INPUTS, "inputs")?;
    count(b.outputs.len(), MAX_OUTPUTS, "outputs")?;
    let reg = &registry::get().bitcoin;
    r.chain = reg.name.clone();
    let key = wallet.key(Network::Bitcoin, r.from.index)?;
    let own_script = bitcoin::p2wpkh_script(&key);

    let mut inputs = Vec::new();
    let mut total_in: u64 = 0;
    for (n, i) in b.inputs.iter().enumerate() {
        let Some(txid) = unhex(&i.txid).filter(|t| t.len() == 32) else {
            bail!("Input {} has an invalid txid.", n + 1)
        };
        let Some(prev_raw) = unhex(&i.prev_tx) else {
            bail!("Input {} has an invalid previous transaction.", n + 1)
        };
        let prev = bitcoin::parse_tx(&prev_raw)?;
        if bitcoin::txid_hex(&prev.txid) != i.txid {
            bail!("Input {}: the previous transaction is not {}.", n + 1, i.txid);
        }
        let Some((value, script)) = prev.outputs.get(i.vout as usize) else {
            bail!("Input {}: transaction {} has no output {}.", n + 1, i.txid, i.vout)
        };
        if i.amount.to_u64() != Some(*value) {
            bail!(
                "Input {}: output {}:{} holds {value} sat, not {}.",
                n + 1,
                i.txid,
                i.vout,
                i.amount.as_str()
            );
        }
        if *script != own_script {
            bail!(
                "Input {}: output {}:{} does not belong to account #{}.",
                n + 1,
                i.txid,
                i.vout,
                r.from.index
            );
        }
        let mut internal: [u8; 32] = txid.try_into().unwrap();
        internal.reverse();
        if inputs.iter().any(|x: &bitcoin::Input| x.txid == internal && x.vout == i.vout) {
            bail!("The request spends {}:{} twice.", i.txid, i.vout);
        }
        total_in = total_in
            .checked_add(*value)
            .ok_or_else(|| crate::Error("The inputs add up to more than exists.".into()))?;
        inputs.push(bitcoin::Input { txid: internal, vout: i.vout, value: *value });
    }

    let mut outputs = Vec::new();
    let (mut total_out, mut sent): (u64, u64) = (0, 0);
    for o in &b.outputs {
        let (address, own) = accounts.resolve(&o.to)?;
        let parsed = bitcoin::parse_address(&address)?;
        let Some(value) = o.amount.to_u64() else { bail!("{} sat is more than exists.", o.amount.as_str()) };
        if value < parsed.kind.dust() {
            bail!(
                "{value} sat to {address} is below the dust limit of {} sat: the network would not relay it.",
                parsed.kind.dust()
            );
        }
        total_out = total_out
            .checked_add(value)
            .ok_or_else(|| crate::Error("The outputs add up to more than exists.".into()))?;
        if own.is_none() {
            sent += value;
        }
        r.transfers.push(Transfer {
            to: address,
            own,
            amount: format_u128(value as u128, 8),
            asset: "BTC".into(),
            token: None,
            verified: true,
            fee: None,
            note: own.map(|_| "Change".to_string()),
        });
        outputs.push(bitcoin::Output { value, script: parsed.script });
    }
    if total_out >= total_in {
        bail!(
            "The outputs ({total_out} sat) are not less than the inputs ({total_in} sat): there is no fee."
        );
    }
    let fee = total_in - total_out;
    // size with 72-byte signatures (DER + sighash type), the common length after low-s
    let vsize = estimate_vsize(&inputs, &outputs);
    let rate = fee as f64 / vsize as f64;
    r.fee = format!("{} BTC", format_u128(fee as u128, 8));
    r.details.push(("Fee rate".into(), format!("≈ {rate:.1} sat/vB")));
    r.details.push(("Size".into(), format!("≈ {vsize} vB")));
    r.details
        .push(("Inputs".into(), format!("{} coins, {} BTC", inputs.len(), format_u128(total_in as u128, 8))));
    // the fee against what is paid to others (against everything when it all stays in the wallet)
    let paid = if sent > 0 { sent } else { total_out };
    if high_fee(fee as u128, paid as u128) || fee / vsize as u64 > HIGH_FEE_RATE {
        r.warnings
            .push(format!("The fee is high: {} BTC (≈ {rate:.0} sat/vB).", format_u128(fee as u128, 8)));
    }
    r.plan = Plan::Bitcoin { inputs, outputs };
    Ok(())
}

fn estimate_vsize(inputs: &[bitcoin::Input], outputs: &[bitcoin::Output]) -> usize {
    let var = |n: usize| if n < 0xfd { 1 } else { 3 };
    let base = 4
        + var(inputs.len())
        + inputs.len() * 41
        + var(outputs.len())
        + outputs.iter().map(|o| 8 + var(o.script.len()) + o.script.len()).sum::<usize>()
        + 4;
    let witness = 2 + inputs.len() * (1 + 1 + 72 + 1 + 33);
    (base * 4 + witness).div_ceil(4)
}

fn to_u128(a: &Amount, what: &str) -> Result<u128> {
    a.to_u128().ok_or_else(|| crate::Error(format!("The {what} {} is too large.", a.as_str())))
}

fn review_evm(b: &EvmSign, accounts: &mut Accounts, r: &mut Review) -> Result<()> {
    count(b.txs.len(), MAX_TXS, "transactions")?;
    let chain = registry::get().evm_chain(b.chain_id);
    let (symbol, decimals) = match chain {
        Some(c) => {
            r.chain = c.name.clone();
            r.testnet = c.testnet;
            (c.symbol.clone(), c.decimals)
        }
        None => {
            r.chain = format!("Chain {}", b.chain_id);
            r.warnings.push(format!(
                "Chain {} is not in Airkeys' list. The amounts are in its native coin, assumed to have 18 decimals. A transaction signed for one chain can only be used on that chain: check the chain id.",
                b.chain_id
            ));
            ("native coin".to_string(), 18)
        }
    };
    r.details.push(("Chain ID".into(), b.chain_id.to_string()));
    let mut txs = Vec::new();
    // native coin: all of it (for overflow), what goes to others, and the highest price per gas
    let (mut total_fee, mut native_sent, mut native_paid, mut top_price): (u128, u128, u128, u128) =
        (0, 0, 0, 0);
    for (k, t) in b.txs.iter().enumerate() {
        if b.txs[0].nonce.checked_add(k as u64) != Some(t.nonce) {
            bail!("The nonces must be consecutive: transaction {} has nonce {}.", k + 1, t.nonce);
        }
        if t.gas < 21000 {
            bail!(
                "Transaction {} has a gas limit of {}: every transaction needs at least 21000.",
                k + 1,
                t.gas
            );
        }
        let fee = match (&t.max_fee_per_gas, &t.max_priority_fee_per_gas, &t.gas_price) {
            (Some(max), Some(prio), None) => {
                let (max_fee, max_priority) = (to_u128(max, "fee")?, to_u128(prio, "fee")?);
                if max_priority > max_fee {
                    bail!("Transaction {}: the priority fee is above the maximum fee.", k + 1);
                }
                evm::Fee::Eip1559 { max_fee, max_priority }
            }
            (None, None, Some(price)) => evm::Fee::Legacy { gas_price: to_u128(price, "gas price")? },
            _ => bail!(
                "Transaction {}: give either maxFeePerGas and maxPriorityFeePerGas, or gasPrice.",
                k + 1
            ),
        };
        top_price = top_price.max(match fee {
            evm::Fee::Eip1559 { max_fee, .. } => max_fee,
            evm::Fee::Legacy { gas_price } => gas_price,
        });
        nonzero(&t.amount)?;
        let (address, own) = accounts.resolve(&t.to)?;
        let to = evm::parse_address(&address)?;
        let to_display = evm::checksum(&to);
        let mut tx = evm::Tx {
            chain_id: b.chain_id,
            nonce: t.nonce,
            gas: t.gas,
            fee,
            to,
            value: Amount::from(0u64),
            data: vec![],
        };
        let transfer = match &t.token {
            None => {
                let value = to_u128(&t.amount, "amount")?;
                native_sent = native_sent
                    .checked_add(value)
                    .ok_or_else(|| crate::Error("The amounts are too large.".into()))?;
                if own.is_none() {
                    native_paid += value;
                }
                tx.value = t.amount.clone();
                Transfer {
                    to: to_display,
                    own,
                    amount: t.amount.format(decimals),
                    asset: symbol.clone(),
                    token: None,
                    verified: true,
                    fee: None,
                    note: None,
                }
            }
            Some(token) => {
                let contract = evm::parse_address(&token.address)?;
                let verified = check_token(chain, token)?;
                if !verified {
                    r.warnings.push(unlisted_warning(token));
                }
                tx.to = contract;
                tx.data = evm::erc20_transfer(&to, &t.amount);
                Transfer {
                    to: to_display,
                    own,
                    amount: t.amount.format(token.decimals),
                    asset: token.symbol.clone(),
                    token: Some(evm::checksum(&contract)),
                    verified,
                    fee: None,
                    note: None,
                }
            }
        };
        let Some(max_fee) = tx.max_fee() else { bail!("Transaction {}: the fee is too large.", k + 1) };
        total_fee =
            total_fee.checked_add(max_fee).ok_or_else(|| crate::Error("The fees are too large.".into()))?;
        r.transfers.push(Transfer {
            fee: Some(format!("≤ {} {symbol}", format_u128(max_fee, decimals))),
            ..transfer
        });
        txs.push(tx);
    }
    r.fee = format!("≤ {} {symbol}", format_u128(total_fee, decimals));
    let nonces = if txs.len() == 1 {
        txs[0].nonce.to_string()
    } else {
        format!("{}–{}", txs[0].nonce, txs[txs.len() - 1].nonce)
    };
    r.details.push(("Nonce".into(), nonces));
    match txs[0].fee {
        evm::Fee::Eip1559 { max_fee, max_priority } => {
            r.details.push(("Max fee per gas".into(), format!("{} gwei", format_u128(max_fee, 9))));
            r.details.push(("Priority fee".into(), format!("{} gwei", format_u128(max_priority, 9))));
        }
        evm::Fee::Legacy { gas_price } => {
            r.details.push(("Gas price".into(), format!("{} gwei", format_u128(gas_price, 9))))
        }
    }
    // the fee against the coin paid to others; and, whatever is paid, a price per gas no
    // chain asks for (moves to this wallet's own accounts are not a payment)
    if high_fee(total_fee, native_paid) || top_price > HIGH_GAS_PRICE {
        r.warnings.push(format!("The fee is high: up to {} {symbol}.", format_u128(total_fee, decimals)));
    }
    r.plan = Plan::Evm(txs);
    Ok(())
}

impl Review {
    /// Signs exactly what was reviewed.
    pub fn sign(&self, wallet: &Wallet) -> Result<Signed> {
        let index = self.from.index;
        let txs = match &self.plan {
            Plan::Bitcoin { inputs, outputs } => {
                let s = bitcoin::sign(&wallet.key(Network::Bitcoin, index)?, inputs, outputs);
                vec![SignedTx { hash: bitcoin::txid_hex(&s.txid), raw: hex(&s.raw) }]
            }
            Plan::Evm(txs) => {
                let key = wallet.key(Network::Evm, index)?;
                txs.iter()
                    .map(|t| {
                        let (raw, hash) = t.sign(&key);
                        SignedTx { hash: format!("0x{}", hex(&hash)), raw: format!("0x{}", hex(&raw)) }
                    })
                    .collect()
            }
        };
        Ok(Signed { id: self.id.clone(), network: self.network, txs, accounts: self.new_accounts.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_threshold() {
        assert!(!high_fee(5, 100));
        assert!(high_fee(6, 100));
        assert!(!high_fee(1_000, 0));
    }
}
