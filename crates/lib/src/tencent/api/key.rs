use super::{Class, Failure};

fn input() -> Failure {
    Failure::new(
        Class::Input,
        "requires bounded OpenSSH RSA public material, 2048 through 4096 bits and a positive odd exponent",
    )
}

pub(in crate::tencent) fn public(value: &str) -> Result<String, Failure> {
    if value.len() > 8192 || value.contains(['\n', '\r']) {
        return Err(input());
    }
    let key = ssh_key::PublicKey::from_openssh(value).map_err(|_| input())?;
    let rsa = key.key_data().rsa().ok_or_else(input)?;
    let modulus = rsa.n.as_positive_bytes().ok_or_else(input)?;
    let bits = modulus.len() * 8 - modulus.first().map_or(0, |b| b.leading_zeros() as usize);
    let exponent = rsa.e.as_positive_bytes().ok_or_else(input)?;
    if !(2048..=4096).contains(&bits) || exponent.len() > 8 {
        return Err(input());
    }
    let exponent = exponent.iter().fold(0u64, |n, b| (n << 8) | u64::from(*b));
    if exponent < 3 || exponent % 2 == 0 {
        return Err(input());
    }
    Ok(key.fingerprint(ssh_key::HashAlg::Sha256).to_string())
}
