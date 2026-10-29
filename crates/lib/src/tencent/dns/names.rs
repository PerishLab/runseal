use super::{Kind, input};
use crate::tencent::Failure;

pub(super) struct Names;

impl Names {
    pub fn id(value: u64) -> Result<(), Failure> {
        if value == 0 {
            return Err(input());
        }
        Ok(())
    }

    pub fn text(value: &str, limit: usize) -> Result<(), Failure> {
        if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
            return Err(input());
        }
        Ok(())
    }

    pub fn labels(value: &str) -> Result<(), Failure> {
        Self::text(value, 253)?;
        for label in value.split('.') {
            let width = (1..=63).contains(&label.len());
            let edges = label.starts_with('-') || label.ends_with('-');
            let lexical = label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
            if !width || edges || !lexical {
                return Err(input());
            }
        }
        Ok(())
    }

    pub fn domain(value: &str) -> Result<(), Failure> {
        Self::labels(value)?;
        if !value.contains('.') {
            return Err(input());
        }
        Ok(())
    }

    pub fn host(value: &str) -> Result<(), Failure> {
        if value == "@" || value == "*" {
            return Ok(());
        }
        let value = value.strip_prefix("*.").unwrap_or(value);
        Self::text(value, 253)?;
        for label in value.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(input());
            }
            if !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            {
                return Err(input());
            }
        }
        Ok(())
    }

    pub fn value(kind: Kind, value: &str) -> Result<(), Failure> {
        match kind {
            Kind::A => {
                let _: std::net::Ipv4Addr = value.parse().map_err(|_| input())?;
            }
            Kind::Aaaa => {
                let _: std::net::Ipv6Addr = value.parse().map_err(|_| input())?;
            }
            Kind::Cname => Self::domain(value.strip_suffix('.').unwrap_or(value))?,
            Kind::Txt => Self::text(value, 4096)?,
        }
        Ok(())
    }
}
