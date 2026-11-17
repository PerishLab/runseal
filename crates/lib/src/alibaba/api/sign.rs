use anyhow::{Result, bail};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub(in crate::alibaba) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn authorization(
    credential: (&str, &str),
    headers: &[(&str, &str)],
    query: &str,
) -> Result<String> {
    let mut entries = headers
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim()))
        .filter(|(name, _)| name == "host" || name == "content-type" || name.starts_with("x-acs-"))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        bail!("Alibaba duplicate signed header refused");
    }
    let names = entries
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let fields = Zeroizing::new(
        entries
            .iter()
            .map(|(name, value)| format!("{name}:{value}\n"))
            .collect::<String>(),
    );
    let canonical = Zeroizing::new(format!(
        "POST\n/\n{query}\n{}\n{names}\n{}",
        fields.as_str(),
        hash(b"")
    ));
    let message = Zeroizing::new(format!("ACS3-HMAC-SHA256\n{}", hash(canonical.as_bytes())));
    let mut mac = Hmac::<Sha256>::new_from_slice(credential.1.as_bytes())
        .map_err(|_| anyhow::anyhow!("Alibaba signature preparation refused"))?;
    mac.update(message.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());
    Ok(format!(
        "ACS3-HMAC-SHA256 Credential={},SignedHeaders={names},Signature={signature}",
        credential.0
    ))
}

pub(super) fn date(timestamp: u64) -> Result<String> {
    let timestamp =
        i64::try_from(timestamp).map_err(|_| anyhow::anyhow!("Alibaba signing time refused"))?;
    let time = time::OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| anyhow::anyhow!("Alibaba signing time refused"))?;
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        time.year(),
        u8::from(time.month()),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    ))
}

#[cfg(test)]
mod tests {
    use super::{authorization, date};

    #[test]
    fn official() {
        let headers = [
            ("host", "ecs.cn-shanghai.aliyuncs.com"),
            ("x-acs-action", "RunInstances"),
            (
                "x-acs-content-sha256",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            ("x-acs-date", "2023-10-26T10:22:32Z"),
            ("x-acs-signature-nonce", "3156853299f313e23d1673dc12e1703d"),
            ("x-acs-version", "2014-05-26"),
        ];
        let query =
            "ImageId=win2019_1809_x64_dtc_zh-cn_40G_alibase_20230811.vhd&RegionId=cn-shanghai";
        let signed =
            authorization(("YourAccessKeyId", "YourAccessKeySecret"), &headers, query).unwrap();
        assert_eq!(
            signed,
            "ACS3-HMAC-SHA256 Credential=YourAccessKeyId,SignedHeaders=host;x-acs-action;x-acs-content-sha256;x-acs-date;x-acs-signature-nonce;x-acs-version,Signature=06563a9e1b43f5dfe96b81484da74bceab24a1d853912eee15083a6f0f3283c0"
        );
    }

    #[test]
    fn canonical() {
        let first = [
            ("Host", "sts.aliyuncs.com"),
            ("X-Acs-Security-Token", " token-a "),
        ];
        let second = [
            ("x-acs-security-token", "token-a"),
            ("host", "sts.aliyuncs.com"),
        ];
        let expected = authorization(("fixture", "secret"), &first, "").unwrap();
        assert_eq!(
            expected,
            authorization(("fixture", "secret"), &second, "").unwrap()
        );
        let other = [
            ("host", "sts.aliyuncs.com"),
            ("x-acs-security-token", "token-b"),
        ];
        assert_ne!(
            expected,
            authorization(("fixture", "secret"), &other, "").unwrap()
        );
        assert!(
            authorization(
                ("fixture", "secret"),
                &[("Host", "one"), ("host", "two")],
                ""
            )
            .is_err()
        );
        assert_eq!(date(0).unwrap(), "1970-01-01T00:00:00Z");
        assert!(date(u64::MAX).is_err());
    }
}
