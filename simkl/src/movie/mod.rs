use reqwest::{
    blocking::ClientBuilder,
    header::{AUTHORIZATION, HeaderMap, USER_AGENT},
};
use serde_json::Value;

use crate::smo::MovieDetails;

pub fn get_movie_details(
    client_id: &str,
    app_name: &str,
    app_version: &str,
    simkl_id: u32,
) -> anyhow::Result<MovieDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("simkl-api-key", client_id.parse().unwrap());
    headers.insert(
        USER_AGENT,
        format!("{app_name}/{app_version}").parse().unwrap(),
    );

    crate::send_request_deserialized(
        &client,
        &format!(
            "https://api.simkl.com/movies/{simkl_id}?app-name={app_name}&app-version={app_version}"
        ),
        Some(&headers),
        None,
        None,
        "Simkl: Error while getting movie details",
    )
}
