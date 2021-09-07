mod errors;
mod locale;
mod scraper;
mod url;

use crate::http::url::PopotamoUrls;
use ::scraper::Html;
use async_trait::async_trait;
use etwin_core::clock::Clock;
use etwin_core::popotamo::{PopotamoClient, PopotamoProfileResponse, PopotamoUserIdRef};
use etwin_core::types::AnyError;
use reqwest::Client;
use std::time::Duration;

const USER_AGENT: &str = "EtwinPopotamoScraper";
const TIMEOUT: Duration = Duration::from_millis(5000);

pub struct HttpPopotamoClient<TyClock> {
  client: Client,
  #[allow(unused)]
  clock: TyClock,
}

impl<TyClock> HttpPopotamoClient<TyClock>
where
  TyClock: Clock,
{
  pub fn new(clock: TyClock) -> Result<Self, AnyError> {
    Ok(Self {
      client: Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()?,
      clock,
    })
  }

  async fn get_html(&self, url: reqwest::Url) -> reqwest::Result<Html> {
    let builder = self.client.get(url);
    let resp = builder.send().await?;
    let text = resp.error_for_status()?.text().await?;
    Ok(Html::parse_document(&text))
  }
}

#[async_trait]
impl<TyClock> PopotamoClient for HttpPopotamoClient<TyClock>
where
  TyClock: Clock,
{
  async fn get_profile(&self, user: PopotamoUserIdRef) -> Result<PopotamoProfileResponse, AnyError> {
    let html = self.get_html(PopotamoUrls::new(user.server).user(user.id)).await?;
    let response = scraper::scrape_profile(&html)?;
    // TODO: Assert username matches
    Ok(response)
  }
}

#[cfg(feature = "neon")]
impl<TyClock> neon::prelude::Finalize for HttpPopotamoClient<TyClock> where TyClock: Clock {}
