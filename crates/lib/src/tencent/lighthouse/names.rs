use super::{Failure, input};

pub(super) struct Names;

impl Names {
    pub fn region(value: &str) -> Result<(), Failure> {
        let parts: Vec<_> = value.split('-').collect();
        if value.len() > 64
            || !(2..=3).contains(&parts.len())
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_lowercase()))
        {
            return Err(input());
        }
        Ok(())
    }

    pub fn id(value: &str, prefix: &str) -> Result<(), Failure> {
        if value.len() > 64
            || value.strip_prefix(prefix).is_none_or(|suffix| {
                suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
        {
            return Err(input());
        }
        Ok(())
    }

    pub fn request(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }

    pub fn zone(value: &str, region: &str) -> bool {
        value.len() <= 64
            && value
                .strip_prefix(&format!("{region}-"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                })
    }
}
