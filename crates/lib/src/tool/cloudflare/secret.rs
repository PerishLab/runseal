use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use serde_json::Value;
use zeroize::Zeroizing;

use crate::custody::Private;
use crate::tool::Reply;

pub(super) struct Reserved {
    file: crate::custody::Reserved,
    path: PathBuf,
    rolls: bool,
}

impl Reserved {
    pub fn open(path: &str, rolls: bool) -> Result<Self> {
        let path = PathBuf::from(path);
        let absolute = path
            .absolutize()
            .with_context(|| format!("unable to resolve token value file {}", path.display()))?;
        let file = Private::current()
            .output(&absolute)
            .with_context(|| format!("unable to reserve token value file {}", path.display()))?;
        Ok(Self { file, path, rolls })
    }

    pub fn commit(self, reply: &mut Reply) -> Result<()> {
        let Some(held) = reply.secret() else {
            anyhow::bail!(
                "cloudflare did not return a token secret; {}",
                failure(reply, &self.path, self.rolls)
            );
        };
        let bytes = Zeroizing::new(format!("{}\n", held.expose()));
        self.file
            .commit(bytes.as_bytes())
            .with_context(|| failure(reply, &self.path, self.rolls))?;
        enrich(&mut reply.value, &self.path);
        Ok(())
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
