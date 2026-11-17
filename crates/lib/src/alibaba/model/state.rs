use super::*;

impl State {
    pub(in crate::alibaba) fn fresh() -> Self {
        Self {
            version: 1,
            site: SITE.into(),
            ..Self::default()
        }
    }

    pub(in crate::alibaba) fn check(&self) -> Result<()> {
        if !matches!(self.version, 1 | 2) || self.site != SITE {
            bail!("Alibaba state version or site refused");
        }
        match (&self.federation, self.version) {
            (None, 1) => (),
            (Some(federation), 2)
                if self.intent.is_none() && self.oauth.is_none() && self.identity.is_none() =>
            {
                federation.check(self.credential.as_ref())?
            }
            _ => bail!("Alibaba OAuth and federated custody modes must remain separate"),
        }
        if let Some(pending) = &self.pending {
            let federated = matches!(pending.stage.as_str(), OIDC | ROLE);
            if federated != self.federation.is_some() {
                bail!("Alibaba pending operation differs from custody mode");
            }
        }
        if let Some(intent) = &self.intent {
            for value in [&intent.state, &intent.verifier, &intent.redirect] {
                text(value)?;
            }
        }
        if let Some(grant) = &self.oauth {
            text(&grant.access)?;
            text(&grant.refresh)?;
        }
        if let Some(credential) = &self.credential {
            text(&credential.id)?;
            text(&credential.key)?;
            text(&credential.token)?;
        }
        if let Some(identity) = &self.identity {
            identity.validate()?;
        }
        if let Some(pending) = &self.pending {
            stage(&pending.stage)?;
            if pending.started == 0 {
                bail!("Alibaba pending stage refused");
            }
        }
        if let Some(retained) = &self.retained {
            stage(&retained.stage)?;
            if self
                .pending
                .as_ref()
                .is_none_or(|pending| pending.stage != retained.stage)
            {
                bail!("Alibaba retained response has no matching pending stage");
            }
            if retained.body.len() > RETAINED {
                bail!("Alibaba retained response exceeds the private state allowance");
            }
            let decoded = Zeroizing::new(
                base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    retained.body.as_bytes(),
                )
                .map_err(|_| anyhow::anyhow!("Alibaba retained response encoding refused"))?,
            );
            if decoded.len() > BODY {
                bail!("Alibaba retained response exceeds the response allowance");
            }
        }
        if let Some(identity) = &self.identity {
            let credential = self.credential.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Alibaba verified identity requires its signing credential")
            })?;
            if identity.stamp != stamp(credential) {
                bail!("Alibaba verified identity no longer matches the stored signing material");
            }
        }
        Ok(())
    }

    pub(in crate::alibaba) fn credential(&self) -> Result<&Credential> {
        let held = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Alibaba STS credentials are absent; renew explicitly")
        })?;
        if held.expires <= now() {
            bail!(
                "Alibaba STS credentials expired; explicitly refresh the OAuth token if needed, renew, then verify identity"
            );
        }
        text(&held.id)?;
        text(&held.key)?;
        text(&held.token)?;
        Ok(held)
    }

    pub(in crate::alibaba) fn receipt(&self) -> Value {
        json!({
            "site": SITE,
            "federation": self.federation.as_ref().map(|held| json!({"binding":held.binding,"tokenPresent":held.token.is_some(),"tokenExpiresAt":held.token.as_ref().map(|token|token.expires),"identity":held.identity})),
            "authorized": self.oauth.is_some(),
            "credentialPresent": self.credential.is_some(),
            "oauthExpiresAt": self.oauth.as_ref().map(|held| held.expires),
            "credentialExpiresAt": self.credential.as_ref().map(|held| held.expires),
            "identity": self.identity,
            "pending": self.pending.as_ref().map(|held| json!({"stage": held.stage, "startedAt": held.started, "retainedReturn": self.retained.is_some()})),
            "callback": self.intent.as_ref().map(|held| json!({"redirect": held.redirect, "expiresAt": held.deadline, "consumed": held.consumed})),
        })
    }
}
