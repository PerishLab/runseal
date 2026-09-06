use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::tool::Reply;

pub(super) struct Reserved {
    file: Option<File>,
    path: PathBuf,
    rolls: bool,
}

impl Reserved {
    pub fn open(path: &str, rolls: bool) -> Result<Self> {
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
            rolls,
        })
    }

    pub fn commit(mut self, reply: &mut Reply) -> Result<()> {
        let Some(held) = reply.secret() else {
            anyhow::bail!(
                "cloudflare did not return a token secret; {}",
                failure(reply, &self.path, self.rolls)
            );
        };
        let secret = held.expose();
        let mut file = self.file.take().expect("reserved file is held");
        let written = file
            .write_all(secret.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all());
        if let Err(error) = written {
            let _ = std::fs::remove_file(&self.path);
            return Err(error).with_context(|| failure(reply, &self.path, self.rolls));
        }
        enrich(&mut reply.value, &self.path);
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

fn enrich(value: &mut Value, path: &Path) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.insert(
        "value_file".into(),
        Value::String(path.display().to_string()),
    );
}

fn failure(reply: &Reply, path: &Path, rolls: bool) -> String {
    let id = reply
        .value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if rolls {
        return format!(
            "cloudflare rolled token {id}, so its previous value is already dead and the new one reached nobody; roll it again at once. Nothing was written to {}",
            path.display()
        );
    }
    format!(
        "cloudflare created token {id}, but its secret could not be written to {}; delete that token, it is unreachable",
        path.display()
    )
}
