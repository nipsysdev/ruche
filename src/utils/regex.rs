use regex::Regex;
use serde::de::{Error, Visitor};

pub const PORT_REGEX: &str = r"^\d{1,3}xx$";
pub const VOLUME_NAME_REGEX: &str = r"^([\w-]+)*[^x]?xx$";

pub struct RegexVisitor(&'static str);

impl RegexVisitor {
    pub(crate) fn new(pattern: &'static str) -> Self {
        Self(pattern)
    }
}

impl<'de> Visitor<'de> for RegexVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string matching pattern: {}", self.0)
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        let regex = Regex::new(self.0).map_err(|_| E::custom("Invalid regex pattern"))?;

        if regex.is_match(value) {
            Ok(value.to_string())
        } else {
            Err(E::custom(format!(
                "Value '{}' doesn't match pattern: {}",
                value, self.0
            )))
        }
    }
}
