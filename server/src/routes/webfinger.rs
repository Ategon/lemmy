use crate::{db::community::Community, routes::DbPoolParam, Settings};
use actix_web::{error::ErrorBadRequest, web::Query, *};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct Params {
  resource: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
  if Settings::get().federation.enabled {
    cfg.route(
      ".well-known/webfinger",
      web::get().to(get_webfinger_response),
    );
  }
}

lazy_static! {
  static ref WEBFINGER_COMMUNITY_REGEX: Regex = Regex::new(&format!(
    "^group:([a-z0-9_]{{3, 20}})@{}$",
    Settings::get().hostname
  ))
  .unwrap();
}

/// Responds to webfinger requests of the following format. There isn't any real documentation for
/// this, but it described in this blog post:
/// https://mastodon.social/.well-known/webfinger?resource=acct:gargron@mastodon.social
///
/// You can also view the webfinger response that Mastodon sends:
/// https://radical.town/.well-known/webfinger?resource=acct:felix@radical.town
async fn get_webfinger_response(
  info: Query<Params>,
  db: DbPoolParam,
) -> Result<HttpResponse, Error> {
  let res = web::block(move || {
    let conn = db.get()?;

    let regex_parsed = WEBFINGER_COMMUNITY_REGEX
      .captures(&info.resource)
      .map(|c| c.get(1))
      .flatten();
    let community_name = match regex_parsed {
      Some(c) => c.as_str(),
      None => return Err(format_err!("not_found")),
    };

    // Make sure the requested community exists.
    let community = match Community::read_from_name(&conn, &community_name) {
      Ok(o) => o,
      Err(_) => return Err(format_err!("not_found")),
    };

    let community_url = community.get_url();

    Ok(json!({
    "subject": info.resource,
    "aliases": [
      community_url,
    ],
    "links": [
      {
        "rel": "http://webfinger.net/rel/profile-page",
        "type": "text/html",
        "href": community.get_url(),
      },
      {
        "rel": "self",
        "type": "application/activity+json",
        "href": community_url
      }
      // TODO: this also needs to return the subscribe link once that's implemented
      //{
      //  "rel": "http://ostatus.org/schema/1.0/subscribe",
      //  "template": "https://my_instance.com/authorize_interaction?uri={uri}"
      //}
    ]
    }))
  })
  .await
  .map(|json| HttpResponse::Ok().json(json))
  .map_err(ErrorBadRequest)?;
  Ok(res)
}
