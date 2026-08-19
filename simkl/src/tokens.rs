use std::sync::mpsc::Sender;

use anyhow::bail;
use reqwest::blocking::ClientBuilder;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct DeviceCodeResponse {
    // result:           String, //           "OK",
    // device_code:      String, //      "DEVICE_CODE",
    user_code:  String, //        "ABCDE",
    // verification_uri: String, // "https://simkl.com/pin",
    // verification_url: String, // "https://simkl.com/pin",
    expires_in: u64, //       900,
    interval:   u64, //         5
}

// https://api.simkl.org/api-reference/pin
pub fn get_tokens(client_id: &str, tx_user_code: Sender<String>) -> anyhow::Result<String> {
    let client = ClientBuilder::new().build()?;

    // Step 1: Request a device code
    let device_code_response = crate::send_request_deserialized::<DeviceCodeResponse>(
        &client,
        &format!("https://api.simkl.com/oauth/pin?client_id={client_id}"),
        None,
        None,
        None,
        "Simkl: Unable to validate user credentials",
    )?;

    // Step 2: Display the code
    _ = tx_user_code.send(device_code_response.user_code.clone());

    // Step 3: Poll for the result
    let mut token_response = crate::send_request_deserialized::<Value>(
        &client,
        &format!(
            "https://api.simkl.com/oauth/pin/{}?client_id={client_id}",
            device_code_response.user_code
        ),
        None,
        None,
        None,
        "",
    )?;
    let mut retries = 0;
    while token_response["result"].as_str().unwrap() != "OK" {
        if retries * device_code_response.interval
            >= device_code_response.expires_in - device_code_response.interval
        {
            bail!("Simkl: couldn't authenticate request token, max retries reached");
        }

        std::thread::sleep(std::time::Duration::from_secs(
            device_code_response.interval,
        ));
        token_response = crate::send_request_deserialized::<Value>(
            &client,
            &format!(
                "https://api.simkl.com/oauth/pin/{}?client_id={client_id}",
                device_code_response.user_code
            ),
            None,
            None,
            None,
            "",
        )?;
        retries += 1;
    }
    drop(tx_user_code);

    // The request token has been approved by the user
    Ok(token_response["access_token"].as_str().unwrap().to_string())
}
