use once_cell::sync::Lazy;
use regex::Regex;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwinoidUserId(String);

impl TwinoidUserId {
  pub const PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[1-9][0-9]{0,8}$").unwrap());

  pub fn try_from_string(raw: String) -> Result<Self, ()> {
    if Self::PATTERN.is_match(&raw) {
      Ok(Self(raw))
    } else {
      Err(())
    }
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwinoidUserDisplayName(String);

impl TwinoidUserDisplayName {
  pub const PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.{1,100}$").unwrap());

  pub fn try_from_string(raw: String) -> Result<Self, ()> {
    if Self::PATTERN.is_match(&raw) {
      Ok(Self(raw))
    } else {
      Err(())
    }
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShortTwinoidUser {
  pub id: TwinoidUserId,
  pub display_name: TwinoidUserDisplayName,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwinoidUserIdRef {
  pub id: TwinoidUserId,
}