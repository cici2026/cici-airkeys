//! Test driver for the state machines.
//!
//! Drives a `Core` exactly the way the wasm bridge and the desktop host do —
//! dispatch an event, collect the shell operations it asked for, resolve them
//! one at a time — so the tests exercise the real correlation plumbing rather
//! than a simulation of it. No device, no browser, no network, no clock.
//!
//! Written against [`SplitEffect`] rather than a concrete effect enum, so a new
//! machine gets the whole driver for free.

#![cfg(feature = "crux")]
#![allow(dead_code)] // each test file uses a different subset

use std::collections::VecDeque;

use crux_core::{App, Core, Request};

use airkeys_core::app::SplitEffect;
use airkeys_core::types::{Item, ItemKind, VaultMeta};

pub struct Driver<A>
where
    A: App + Default,
    A::Model: Default,
    A::Effect: SplitEffect,
{
    app: Core<A>,
    pending: VecDeque<Request<<A::Effect as SplitEffect>::Op>>,
}

impl<A> Driver<A>
where
    A: App + Default,
    A::Model: Default,
    A::Effect: SplitEffect,
    <A::Effect as SplitEffect>::Op: Clone,
{
    pub fn new() -> Self {
        Self {
            app: Core::new(),
            pending: VecDeque::new(),
        }
    }

    /// Send an event; returns the operations the core asked for, in order.
    pub fn dispatch(&mut self, event: A::Event) -> Vec<<A::Effect as SplitEffect>::Op> {
        let effects = self.app.process_event(event);
        self.collect(effects)
    }

    /// Answer the oldest outstanding operation.
    ///
    /// Panics when nothing is outstanding — a test that resolves into thin air
    /// is asserting something that never happened.
    pub fn resolve(
        &mut self,
        result: <<A::Effect as SplitEffect>::Op as crux_core::capability::Operation>::Output,
    ) -> Vec<<A::Effect as SplitEffect>::Op> {
        let mut request = self
            .pending
            .pop_front()
            .expect("no outstanding shell operation to resolve");
        match self.app.resolve(&mut request, result) {
            Ok(effects) => self.collect(effects),
            // An aborted command's late result: expected, and a no-op.
            Err(_) => Vec::new(),
        }
    }

    /// Answer the oldest outstanding operation matching `predicate`, leaving
    /// the others outstanding — for the cases where two operations are
    /// legitimately in flight and the ORDER they answer in is the thing under
    /// test (the elapsed ticker racing the biometric sheet, say).
    pub fn resolve_matching(
        &mut self,
        predicate: impl Fn(&<A::Effect as SplitEffect>::Op) -> bool,
        result: <<A::Effect as SplitEffect>::Op as crux_core::capability::Operation>::Output,
    ) -> Vec<<A::Effect as SplitEffect>::Op> {
        let index = self
            .pending
            .iter()
            .position(|request| predicate(&request.operation))
            .expect("no outstanding shell operation matches");
        let mut request = self
            .pending
            .remove(index)
            .expect("index came from position");
        match self.app.resolve(&mut request, result) {
            Ok(effects) => self.collect(effects),
            Err(_) => Vec::new(),
        }
    }

    pub fn view(&self) -> A::ViewModel {
        self.app.view()
    }

    /// Operations still awaiting an answer.
    pub fn outstanding(&self) -> Vec<<A::Effect as SplitEffect>::Op> {
        self.pending
            .iter()
            .map(|request| request.operation.clone())
            .collect()
    }

    /// Drop the oldest outstanding operation unanswered — "the shell never got
    /// back to us". Used to assert what happens to abandoned requests.
    pub fn drop_oldest(&mut self) {
        self.pending.pop_front();
    }

    fn collect(&mut self, effects: Vec<A::Effect>) -> Vec<<A::Effect as SplitEffect>::Op> {
        let mut operations = Vec::new();
        for effect in effects {
            if let Some(request) = effect.into_shell() {
                operations.push(request.operation.clone());
                self.pending.push_back(request);
            }
        }
        operations
    }
}

impl<A> Default for Driver<A>
where
    A: App + Default,
    A::Model: Default,
    A::Effect: SplitEffect,
    <A::Effect as SplitEffect>::Op: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

pub const NOW: &str = "2026-08-12T09:00:00.000Z";

pub fn vault(item_count: u32) -> VaultMeta {
    VaultMeta {
        id: "vault-1".into(),
        label: "Personal".into(),
        item_count,
        created_at_iso: "2026-01-04T10:00:00.000Z".into(),
    }
}

pub fn item(id: &str, title: &str, account: &str, site: &str) -> Item {
    Item {
        id: id.into(),
        kind: ItemKind::Login,
        title: title.into(),
        account: account.into(),
        site: site.into(),
        favorite: false,
        updated_at_iso: NOW.into(),
    }
}
