use crate::config::TwitchCredentials;
use crate::error::{Result, TwitchError};
use serde::{Deserialize, Serialize};

const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";

/// Response from the token refresh endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    /// New access token
    pub access_token: String,
    /// New refresh token (may be the same as the old one)
    pub refresh_token: String,
    /// Number of seconds until the access token expires
    pub expires_in: u32,
    /// List of OAuth scopes granted to the token
    pub scope: Vec<String>,
    /// Type of token (usually "bearer")
    pub token_type: String,
}

/// Refresh the access token using a refresh token
///
/// # Arguments
/// * `refresh_token` - The refresh token to use for getting a new access token
/// * `credentials` - The Twitch application credentials
///
/// # Returns
/// A `TokenResponse` containing the new access token and refresh token
///
/// # Errors
/// Returns an error if `client_secret` is not provided or if the refresh request fails
///
/// # OAuth Scopes
/// Uses the scopes that were originally granted to the refresh token
pub async fn refresh_access_token(
    refresh_token: &str,
    credentials: &TwitchCredentials,
) -> Result<TokenResponse> {
    let client_secret = credentials
        .client_secret
        .as_ref()
        .ok_or_else(|| {
            TwitchError::AuthError(
                "client_secret is required for automatic token refresh".to_string(),
            )
        })?;

    let client = reqwest::Client::new();

    let params = [
        ("client_id", credentials.client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = client.post(TOKEN_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(TwitchError::AuthError(format!(
            "Token refresh failed: HTTP {} - {}",
            status, error_text
        )));
    }

    let token_response = response.json::<TokenResponse>().await?;

    Ok(token_response)
}

/// Validate the current access token
///
/// # Arguments
/// * `access_token` - The access token to validate
///
/// # Returns
/// `true` if the token is valid, `false` otherwise
#[allow(dead_code)] // Public API for token validation
pub async fn validate_token(access_token: &str) -> Result<bool> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {}", access_token))
        .send()
        .await?;

    Ok(response.status().is_success())
}
