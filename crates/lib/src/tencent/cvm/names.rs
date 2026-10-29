use super::{Failure, input};

pub(super) struct Names;

impl Names {
    pub fn text(value: &str, limit: usize) -> Result<(), Failure> {
        if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
            return Err(input());
        }
        Ok(())
    }

    pub fn token(value: &str) -> Result<(), Failure> {
        Self::text(value, 64)?;
        if !value.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(input());
        }
        Ok(())
    }

    pub fn lexical(value: &str) -> Result<(), Failure> {
        Self::text(value, 64)?;
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
        {
            return Err(input());
        }
        Ok(())
    }

    pub fn id(value: &str, prefix: &str) -> Result<(), Failure> {
        Self::lexical(value)?;
        let suffix = value.strip_prefix(prefix).ok_or_else(input)?;
        if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(input());
        }
        Ok(())
    }

    pub fn region(value: &str) -> Result<(), Failure> {
        Self::lexical(value)?;
        if !value.contains('-') || value.ends_with('-') {
            return Err(input());
        }
        Ok(())
    }

    pub fn zone(value: &str, region: &str) -> Result<(), Failure> {
        Self::region(region)?;
        let suffix = value
            .strip_prefix(region)
            .and_then(|v| v.strip_prefix('-'))
            .ok_or_else(input)?;
        if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(input());
        }
        Ok(())
    }

    pub fn request(value: &str) -> Result<(), Failure> {
        Self::text(value, 128)?;
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(input());
        }
        Ok(())
    }

    pub fn unique(values: &[String], prefix: &str) -> Result<(), Failure> {
        if values.len() > 100 {
            return Err(input());
        }
        let mut seen = std::collections::BTreeSet::new();
        for value in values {
            Self::id(value, prefix)?;
            if !seen.insert(value) {
                return Err(input());
            }
        }
        Ok(())
    }
}
