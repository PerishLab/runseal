use super::input;
use crate::tencent::Failure;
use ipnet::Ipv4Net;

pub(super) struct Names;

impl Names {
    pub fn text(value: &str, limit: usize) -> Result<(), Failure> {
        if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
            return Err(input());
        }
        Ok(())
    }

    pub fn region(value: &str) -> Result<(), Failure> {
        let lexical = value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        let parts: Vec<_> = value.split('-').collect();
        if !lexical || value.len() > 64 || parts.len() < 2 {
            return Err(input());
        }
        if parts.iter().any(|part| part.is_empty()) {
            return Err(input());
        }
        Ok(())
    }

    pub fn id(value: &str, prefix: &str) -> Result<(), Failure> {
        let suffix = value.strip_prefix(prefix).ok_or_else(input)?;
        if !(1..=64).contains(&suffix.len())
            || !suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            return Err(input());
        }
        Ok(())
    }

    pub fn cidr(value: &str, private: bool) -> Result<(), Failure> {
        let network = value.parse::<Ipv4Net>().map_err(|_| input())?;
        if network.addr() != network.network() {
            return Err(input());
        }
        if private && (!network.addr().is_private() || !(16..=28).contains(&network.prefix_len())) {
            return Err(input());
        }
        Ok(())
    }
}
