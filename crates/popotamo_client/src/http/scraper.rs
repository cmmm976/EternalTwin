use etwin_core::popotamo::PopotamoSubProfile;
use etwin_core::popotamo::PopotamoUserItem;
use crate::http::errors::ScraperError;
use crate::http::url::PopotamoUrls;
use etwin_core::popotamo::PopotamoSubProfileId;
use etwin_core::popotamo::{
  PopotamoProfile, PopotamoProfileResponse, PopotamoServer, PopotamoSessionUser, PopotamoUserId, PopotamoUsername,
  ShortPopotamoUser,
};
use etwin_scraper_tools::selector;
use etwin_scraper_tools::ElementRefExt;
use itertools::Itertools;
use scraper::{ElementRef, Html, Selector};
use std::str::FromStr;

#[derive(Debug)]

pub(crate) struct ScraperContext {
  server: PopotamoServer,
  session: Option<PopotamoSessionUser>,
}

fn scrape_context(doc: ElementRef) -> Result<ScraperContext, ScraperError> {
  // Only the french server is supported
  let server = PopotamoServer::PopotamoCom;

  let session = doc
    .select(selector!("#menu table#sheet"))
    .at_most_one()
    .map_err(|_| ScraperError::DuplicateSessionBox)?;

  let session = if let Some(session) = session {
    let rewards = session
      .select(selector!(":scope div.rewards"))
      .exactly_one()
      .map_err(|_| ScraperError::NonUniqueSessionUserRewards)?;
    let user_link = rewards
      .select(selector!(":scope a"))
      .next()
      .ok_or(ScraperError::MissingSessionUserLink)?;

    let user_id_href = user_link.value().attr("href").ok_or(ScraperError::MissingLinkHref)?;
    let user_id = PopotamoUrls::new(server)
      .parse_from_root(user_id_href)
      .map_err(|_| ScraperError::InvalidUserLink(user_id_href.to_string()))?;
    let user_id: Option<&str> = user_id
      .path_segments()
      .into_iter()
      .flat_map(|mut segments| segments.nth(1))
      .next();
    let user_id = user_id.ok_or_else(|| ScraperError::InvalidUserLink(user_id_href.to_string()))?;
    let user_id = PopotamoUserId::from_str(user_id).map_err(|_| ScraperError::InvalidUserId(user_id.to_string()))?;

    let username = user_link
      .get_one_text()
      .map_err(|_| ScraperError::NonUniqueSessionUserLinkText)?;
    let username: PopotamoUsername = username
      .parse()
      .map_err(|_| ScraperError::InvalidUsername(username.to_string()))?;

    // let mut test_sub_profile: Vec<PopotamoSubProfile> = Vec::new();
    // let item: PopotamoUserItem = "poubelle"
    //   .parse()
    //   .map_err(|_| ScraperError::InvalidItemName("poubelle".to_string()))?;

    // let items: Vec<PopotamoUserItem> = vec![item];

    // test_sub_profile.push(PopotamoSubProfile { items: items });

    Some(PopotamoSessionUser {
      user: ShortPopotamoUser {
        server,
        id: user_id,
        username,
      },
      // sub_profiles : test_sub_profile,
    })
  } else {
    None
  };

  Ok(ScraperContext { server, session })
}

pub(crate) fn scrape_id(doc: &Html, scraper_context: &ScraperContext) -> Result<PopotamoUserId,ScraperError>{

  let profile_user_id_link = doc
    .select(selector!("a.position"))
    .next()
    .ok_or( ScraperError::MissingProfileUserIdLink)?;

  let profile_user_id_href = profile_user_id_link.value().attr("href").ok_or(ScraperError::MissingLinkHref)?;

  let profile_user_id_url = PopotamoUrls::new(scraper_context.server)
    .parse_from_root(profile_user_id_href)
    .map_err(|_| ScraperError::InvalidUserLink(profile_user_id_href.to_string()))?;

  let profile_user_id_url_string: String = profile_user_id_url.to_string();

  let profile_user_id: Option<&str> = profile_user_id_url_string
    .split("=")
    .nth(1);

    
  let profile_user_id = profile_user_id.ok_or_else(|| ScraperError::InvalidUserLink(profile_user_id_href.to_string()))?;
  let profile_user_id = PopotamoUserId::from_str(profile_user_id).map_err(|_| ScraperError::InvalidUserId(profile_user_id.to_string()))?;

  Ok(profile_user_id)
}

pub(crate) fn scrape_username(doc: &Html) -> Result<PopotamoUsername,ScraperError> {
  
  let profile_username_h2 = doc
    .select(selector!("h2.mainsheet"))
    .exactly_one()
    .map_err(|_| ScraperError::MissingH2Selector)?;

  let profile_username_text = profile_username_h2
    .text()
    .nth(5)
    .ok_or(ScraperError::MissingProfileUsername)?;

  
  let profile_username_no_spaces = profile_username_text
      .split(" ")
      .nth(1)
      .ok_or(ScraperError::MissingProfileUsername)?;

  let profile_username_clean = profile_username_no_spaces
    .split("\n")
    .nth(0)
    .ok_or(ScraperError::MissingProfileUsername)?;


  let profile_username: PopotamoUsername = profile_username_clean
    .parse()
    .map_err(|_| ScraperError::InvalidUsername(profile_username_clean.to_string()))?;

  Ok(profile_username)

}

pub(crate) fn scrape_items(sub_profile_div: &ElementRef) -> Result<Vec<PopotamoUserItem>,ScraperError>{
  let profile_items_td = sub_profile_div
    .select(selector!("td.opt"))
    .exactly_one()
    .map_err(|_| ScraperError::MissingProfileUserItems)?;

  let mut items: Vec<PopotamoUserItem> = Vec::new();

  for element in profile_items_td.select(selector!("img")){

    let ref_item = element.value().attr("alt").ok_or(ScraperError::MissingProfileUserItem)?;

    let item: PopotamoUserItem = ref_item
      .parse()
      .map_err(|_| ScraperError::InvalidItemName(ref_item.to_string()))?;

    items.push(item);

  }

  Ok(items)
}




pub(crate) fn scrape_sub_profiles(doc: &Html) -> Result<Vec<PopotamoSubProfile>,ScraperError>
{
  let mut sub_profiles: Vec<PopotamoSubProfile> = Vec::new();

  let selector = Selector::parse("div[id^='profile_']").unwrap();

  //TO DO : case if user doesn't have sub profiles
  let sub_profile_divs = doc.select(&selector);

  for sub_profile_div in sub_profile_divs{

    let items: Vec<PopotamoUserItem> = scrape_items(&sub_profile_div)?;

    sub_profiles.push(PopotamoSubProfile{items : items});
    
  }

  Ok(sub_profiles)
}

pub(crate) fn scrape_profile(doc: &Html) -> Result<PopotamoProfileResponse, ScraperError> {
  
  let root = doc.root_element();

  let scraper_context = scrape_context(root)?;
  
  let profile = PopotamoProfile {
    user: ShortPopotamoUser {
      server: scraper_context.server,
      id: scrape_id(doc,&scraper_context)?,
      username: scrape_username(doc)?,
      
    },
    sub_profiles : scrape_sub_profiles(doc)?,

  };

  Ok(PopotamoProfileResponse {
    session_user: scraper_context.session,
    profile,
  })
}

#[cfg(test)]
mod test {
  use crate::http::scraper::scrape_profile;
  use etwin_core::popotamo::PopotamoProfileResponse;
  use scraper::Html;
  use std::path::{Path, PathBuf};
  use test_generator::test_resources;

  #[test_resources("./test-resources/scraping/popotamo/user/*/")]
  fn test_scrape_profile(path: &str) 
  {
    let path: PathBuf = Path::join(Path::new("../.."), path);
    let value_path = path.join("value.json");
    let html_path = path.join("main.html");
    let actual_path = path.join("rs.actual.json");

    let raw_html = ::std::fs::read_to_string(html_path).expect("Failed to read html file");

    let html = Html::parse_document(&raw_html);

    let actual = scrape_profile(&html).unwrap();
    let actual_json = serde_json::to_string_pretty(&actual).unwrap();
    ::std::fs::write(actual_path, format!("{}\n", actual_json)).expect("Failed to write actual file");

    let value_json = ::std::fs::read_to_string(value_path).expect("Failed to read value file");
    let expected = serde_json::from_str::<PopotamoProfileResponse>(&value_json).expect("Failed to parse value file");

    assert_eq!(actual, expected);
  }
}
