use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::tool::Reply;

pub(super) struct Reserved {
    file: Option<File>,
    path: PathBuf,
}

impl Reserved {
    pub fn open(path: &str) -> Result<Self> {
        let path = PathBuf::from(path);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options
            .open(&path)
            .with_context(|| format!("unable to reserve token value file {}", path.display()))?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }

    pub fn commit(mut self, reply: &mut Reply) -> Result<()> {
        let secret = reply
            .secret()
            .context("cloudflare did not return a token secret")?
            .expose();
        let digest = format!("{:x}", Sha256::digest(secret.as_bytes()));
        let mut file = self.file.take().expect("reserved file is held");
        let written = file
            .write_all(secret.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all());
        if let Err(error) = written {
            let _ = std::fs::remove_file(&self.path);
            return Err(error).with_context(|| failure(reply, &self.path));
        }
        enrich(&mut reply.value, &self.path, &digest);
        Ok(())
    }
}

impl Drop for Reserved {
    fn drop(&mut self) {
        if self.file.is_some() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn enrich(value: &mut Value, path: &Path, digest: &str) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.insert(
        "value_file".into(),
        Value::String(path.display().to_string()),
    );
    object.insert("value_sha256".into(), Value::String(digest.to_string()));
}

fn failure(reply: &Reply, path: &Path) -> String {
    let id = reply
        .value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!(
        "cloudflare created or rolled token {id}, but its secret could not be written to {}",
        path.display()
    )
}
