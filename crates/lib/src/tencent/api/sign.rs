use anyhow::{Result, bail};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::super::model::Credential;

pub(in crate::tencent) fn authorization(
    credential: &Credential,
    target: (&str, &str, &str),
    body: &str,
    timestamp: u64,
) -> Result<String> {
    let (host, service, action) = target;
    let date = time::OffsetDateTime::from_unix_timestamp(
        timestamp
            .try_into()
            .map_err(|_| anyhow::anyhow!("Tencent clock refused"))?,
    )
    .map_err(|_| anyhow::anyhow!("Tencent clock refused"))?
    .date();
    let date = format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    );
    let signed = "content-type;host;x-tc-action";
    let action = action.to_ascii_lowercase();
    let canonical = format!(
        "POST\n/\n\ncontent-type:application/json; charset=utf-8\nhost:{host}\nx-tc-action:{action}\n\n{signed}\n{}",
        hash(body.as_bytes())
    );
    let scope = format!("{date}/{service}/tc3_request");
    let message = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        hash(canonical.as_bytes())
    );
    let seed = Zeroizing::new(format!("TC3{}", credential.key));
    let day = Zeroizing::new(mac(seed.as_bytes(), date.as_bytes())?);
    let product = Zeroizing::new(mac(&day, service.as_bytes())?);
    let signing = Zeroizing::new(mac(&product, b"tc3_request")?);
    let signature = mac(&signing, message.as_bytes())?;
    let signature: String = signature.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "TC3-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed}, Signature={signature}",
        credential.id
    ))
}

fn mac(key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key) else {
        bail!("Tencent signing refused");
    };
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub(in crate::tencent) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
