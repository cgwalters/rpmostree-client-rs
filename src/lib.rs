//! APIs for interacting with rpm-ostree client side.
//! Currently, this only supports read-only introspection
//! of system state.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//! let status = rpmostree_client::query_status()?;
//! for deployment in status.deployments {
//!     let booted_star = if deployment.booted { "* " } else { "" };
//!     println!("{}Commit: {}", booted_star, deployment.checksum);
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::Context;
use serde_derive::Deserialize;
use std::process::Command;
use thiserror::Error;

/// Our generic catchall fatal error, expected to be converted
/// to a string to output to a terminal or logs.
#[derive(Error, Debug)]
#[error("{0}")]
pub struct Error(String);

/// Representation of the rpm-ostree client-side state; this
/// can be parsed directly from the output of `rpm-ostree status --json`.
/// Currently not all fields are here, but that is a bug.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Status {
    pub deployments: Vec<Deployment>,
}

/// A single deployment, i.e. a bootable ostree commit
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Deployment {
    pub unlocked: Option<String>,
    pub osname: String,
    pub pinned: bool,
    pub checksum: String,
    pub staged: Option<bool>,
    pub booted: bool,
    pub serial: u32,
    pub origin: String,
}

/// Gather a snapshot of the system status.
fn impl_query_status() -> anyhow::Result<Status> {
    // Retry on temporary activation failures, see
    // https://github.com/coreos/rpm-ostree/issues/2531
    let pause = std::time::Duration::from_secs(1);
    let max_retries = 10;
    let mut retries = 0;
    let cmd_res = loop {
        retries += 1;
        let res = Command::new("rpm-ostree")
            .args(&["status", "--json"])
            .output()
            .context("failed to spawn 'rpm-ostree status'")?;

        if res.status.success() || retries >= max_retries {
            break res;
        }
        std::thread::sleep(pause);
    };

    if !cmd_res.status.success() {
        anyhow::bail!(
            "running 'rpm-ostree status' failed: {}",
            String::from_utf8_lossy(&cmd_res.stderr)
        )
    }

    Ok(serde_json::from_slice(&cmd_res.stdout)
        .context("failed to parse 'rpm-ostree status' output")?)
}

pub fn query_status() -> Result<Status, Error> {
    impl_query_status().map_err(|e| Error(e.to_string()))
}
