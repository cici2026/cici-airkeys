//! Emit the TypeScript mirrors of the Core ↔ Shell wire types.
//!
//! The boundary is JSON, so nothing but this generator keeps the two sides
//! honest about the operation and result unions. A drifted variant fails at run
//! time, in a branch the end-to-end suites rarely reach — the locked-out
//! authenticator, the storage read that failed — so the output is committed and
//! drift-gated, the same way every other generated artifact in this repo is.
//!
//! Run through `node rust/scripts/gen-core-types.mjs` (which adds the `--check`
//! gate), not directly.

use std::{env, error::Error, fs, path::PathBuf};

use ts_rs::{Config, TS};

use airkeys_core::app::item_list::{Event as ItemListEvent, ItemListView};
use airkeys_core::app::shell::{ShellOperation, ShellResult};
use airkeys_core::app::unlock::{Event as UnlockEvent, UnlockView};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = match env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
            .join("../../../app-web/airkeys.app/src/lib/core/generated"),
    };
    fs::create_dir_all(&out_dir)?;

    // `export_all` walks nested types, so these six roots cover the whole
    // surface: events in, operations out, results back, view models rendered.
    let config = Config::new().with_out_dir(&out_dir);
    UnlockEvent::export_all(&config)?;
    UnlockView::export_all(&config)?;
    ItemListEvent::export_all(&config)?;
    ItemListView::export_all(&config)?;
    ShellOperation::export_all(&config)?;
    ShellResult::export_all(&config)?;

    println!("core bindings written to {}", out_dir.display());
    Ok(())
}
