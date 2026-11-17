use super::Binding;
use anyhow::{Result, bail};
use radicula_oidc::jwt::{Algorithm, Audience, Claims, DecodingKey, Rules, Verifier};
use serde_json::Value;

pub(super) fn verify(token: &str, keys: &Value, binding: &Binding) -> Result<Claims> {
    let keys = keys
        .get("keys")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Ensign JWKS is missing"))?;
    if keys.is_empty() || keys.len() > 32 {
        bail!("Ensign JWKS key count refused");
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut verified = None;
    for key in keys {
        let kid = field(key, "kid")?;
        if !seen.insert(kid) {
            bail!("Ensign JWKS has duplicate key identifiers");
        }
        let verifier = verifier(key)?;
        let rules = Rules {
            issuer: &binding.issuer,
            audience: &binding.audience,
            subject: Some(&binding.subject),
            now: super::super::model::now(),
            leeway: 0,
            required: &[("kind", "access")],
        };
        if let Ok(claims) = verifier.verify(token, &rules) {
            if verified.is_some() {
                bail!("Ensign token has ambiguous verification keys");
            }
            verified = Some(claims);
        }
    }
    let claims = verified.ok_or_else(|| {
        anyhow::anyhow!("Ensign token signature, issuer, subject, audience or lifetime refused")
    })?;
    if !matches!(&claims.aud, Audience::One(audience) if audience == &binding.audience) {
        bail!("Alibaba federation requires one exact token audience");
    }
    Ok(claims)
}

fn verifier(key: &Value) -> Result<Verifier> {
    if field(key, "use")? != "sig" {
        bail!("Ensign JWKS key purpose refused");
    }
    let (algorithm, decoded) = match (field(key, "kty")?, field(key, "alg")?) {
        ("EC", "ES256") if field(key, "crv")? == "P-256" => (
            Algorithm::ES256,
            DecodingKey::from_ec_components(field(key, "x")?, field(key, "y")?)?,
        ),
        ("RSA", "RS256") => (
            Algorithm::RS256,
            DecodingKey::from_rsa_components(field(key, "n")?, field(key, "e")?)?,
        ),
        _ => bail!("Ensign JWKS algorithm refused"),
    };
    Ok(Verifier {
        algorithm,
        kid: field(key, "kid")?.into(),
        key: decoded,
    })
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    super::super::model::field(value, name)
}
