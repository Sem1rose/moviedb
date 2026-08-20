use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Deserializer};

fn parse_string<'de, D: Deserializer<'de>, T: Default + FromStr>(d: D) -> Result<T, D::Error>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    Deserialize::deserialize(d).and_then(|value: Option<&str>| {
        value.map_or(Ok(Default::default()), |value| {
            value.parse().map_err(serde::de::Error::custom)
        })
    })
}
fn parse_nullable_string<'de, D: Deserializer<'de>, T: FromStr>(d: D) -> Result<Option<T>, D::Error>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    Deserialize::deserialize(d).and_then(|value: Option<&str>| {
        value.map_or(Ok(None), |value| {
            value
                .parse::<T>()
                .map_err(serde::de::Error::custom)
                .map(Option::Some)
        })
    })
}

macro_rules! headers {
    ($client_id:expr, $app_name:expr, $app_version:expr $(, access_token: $access_token:expr)? $(, ($extra_header_name:expr, $extra_header_value:expr))*) => {
        {
            let mut headers = HeaderMap::new();
            headers.insert("simkl-api-key", $client_id.parse().unwrap());
            headers.insert(
                USER_AGENT,
                format!("{}/{}", $app_name, $app_version).parse().unwrap(),
            );
            $(
                headers.insert(AUTHORIZATION, format!("Bearer {}", $access_token).parse().unwrap());
            )?
            $(
                headers.insert($extra_header_name, $extra_header_value.parse().unwrap());
            )*

            headers
        }
    };
}

macro_rules! query {
    ($app_name:expr, $app_version:expr $(, $extra_query:expr)*) => {
        [("app-name", $app_name), ("app-version", $app_version) $(, $extra_query)*]
    };
}

#[derive(Deserialize, Debug)]
pub struct ResponseError {
    pub error:   String,
    pub code:    u32,
    pub message: Option<String>,
}
impl std::error::Error for ResponseError {}
impl Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error {} {}: {}",
            self.code,
            self.error,
            self.message.as_ref().unwrap_or(&"no message".into()),
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct Rating {
    pub rating: f64,
    pub votes:  usize,
}

#[derive(Deserialize, Debug)]
pub struct Ratings {
    pub imdb: Rating,
    // pub simkl: Rating,
}

#[derive(Deserialize, Debug)]
pub struct Ids {
    #[serde(alias = "simkl_id")]
    pub simkl:      u32,
    pub slug:       String,
    pub imdb:       Option<String>,
    pub letterboxd: Option<String>,
    #[serde(deserialize_with = "parse_nullable_string", default)]
    pub tmdb:       Option<u32>,
    pub traktslug:  Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub ids:       Ids,
    pub poster:    String,
    pub title:     String,
    #[serde(alias = "type", alias = "endpoint_type")]
    pub item_type: String,
    pub year:      u32,
    pub ratings:   Option<Ratings>,
}
