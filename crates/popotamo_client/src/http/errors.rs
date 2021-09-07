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
  MissingIDAttribute,
  #[error("Missing handicap td selector while scraping user sub profile")]
  MissingHandicapTDSelector,
  #[error("Missing an handicap value on user profile")]
  MissingHandicapValue,
  #[error("Invalid handicap value {:?}", .0)]
  InvalidHandicapValue(String),
  #[error("Missing game played td selector while scraping user sub profile")]
  MissingGamePlayedTDSelector,
  #[error("Missing an game played value on user profile")]
  MissingGamePlayedValue,
  #[error("Invalid game played value {:?}", .0)]
  InvalidGamePlayedValue(String),
  #[error("Missing speed div selector while scraping user sub profile")]
  MissingSpeedDivSelector,
  #[error("Missing a speed value on user profile")]
  MissingSpeedValue,
  #[error("Invalid speed value {:?}", .0)]
  InvalidSpeedValue(String),
  #[error("Missing creativity div selector while scraping user sub profile")]
  MissingCreativityDivSelector,
  #[error("Missing a creativity value on user profile")]
  MissingCreativityValue,
  #[error("Invalid creativity value {:?}", .0)]
  InvalidCreativityValue(String),
  #[error("Missing wisdom div selector while scraping user sub profile")]
  MissingWisdomDivSelector,
  #[error("Missing a wisdom value on user profile")]
  MissingWisdomValue,
  #[error("Invalid wisdom value {:?}", .0)]
  InvalidWisdomValue(String),
  #[error("Missing efficiency on user profile")]
  MissingProfileEfficiency,
  #[error("Missing efficiency value")]
  MissingEfficiencyValue,
  #[error("Invalid efficiency value {:?}", .0)]
  InvalidEfficiencyValue(String),
}
