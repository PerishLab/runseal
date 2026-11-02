use super::{Failure, input};
pub(super) struct Names;
impl Names {
    pub fn lexical(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
    }

    pub fn region(value: &str) -> Result<(), Failure> {
        let parts: Vec<_> = value.split('-').collect();
        if !(2..=3).contains(&parts.len())
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_lowercase()))
        {
            return Err(input());
        }
        if value.len() > 64 {
            return Err(input());
        }
        Ok(())
    }

    pub fn instance(value: &str) -> Result<(), Failure> {
        if !Self::lexical(value)
            || value.strip_prefix("lhins-").is_none_or(|suffix| {
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
}
