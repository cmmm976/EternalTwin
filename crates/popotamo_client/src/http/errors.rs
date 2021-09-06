use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScraperError {
  #[error("Duplicate session box, expected zero or one")]
  DuplicateSessionBox,
  #[error("Non-unique session-user rewards, expected exactly one")]
  NonUniqueSessionUserRewards,
  #[error("Missing session-user link, expected exactly one")]
  MissingSessionUserLink,
  #[error("Non-unique session user link text node, expected exactly one")]
  NonUniqueSessionUserLinkText,
  #[error("Invalid user id {:?}", .0)]
  InvalidUserId(String),
  #[error("Invalid username {:?}", .0)]
  InvalidUsername(String),
  #[error("Missing href attribute on link")]
  MissingLinkHref,
  #[error("Invalid user link {:?}", .0)]
  InvalidUserLink(String),
  #[error("HTTP Error")]
  HttpError(#[from] reqwest::Error),
  #[error("Missing profile user link")]
  MissingProfileUserIdLink,
  #[error("Missing h2 selector on user page")]
  MissingH2Selector,
  #[error("Missing profile username")]
  MissingProfileUsername,
  #[error("Missing profile user items")]
  MissingProfileUserItems,
  #[error("Missing profile user item")]
  MissingProfileUserItem,
  #[error("Invalid item name {:?}", .0)]
  InvalidItemName(String),
  #[error("Invalid sub profile id {:?}", .0)]
  InvalidSubProfileId(String),
  #[error("Iterator failed. Index might have not been found into it.")]
  IteratorError,
  #[error("Missing ID attribute on sub profile div")]
  MissingIDAttribute
}
