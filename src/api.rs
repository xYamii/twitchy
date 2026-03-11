use crate::auth;
use crate::config::TwitchCredentials;
use crate::error::{Result, TwitchError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

const CHAT_MESSAGES_URL: &str = "https://api.twitch.tv/helix/chat/messages";
const MODERATION_CHAT_URL: &str = "https://api.twitch.tv/helix/moderation/chat";
const MODERATION_BANS_URL: &str = "https://api.twitch.tv/helix/moderation/bans";
#[allow(dead_code)] // Reserved for future chat settings management
const CHAT_SETTINGS_URL: &str = "https://api.twitch.tv/helix/chat/settings";
const USERS_URL: &str = "https://api.twitch.tv/helix/users";

/// Response from sending a chat message
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageResponse {
    pub data: Vec<SendMessageData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageData {
    #[allow(dead_code)] // Part of Twitch API response
    pub message_id: String,
    pub is_sent: bool,
    pub drop_reason: Option<DropReason>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DropReason {
    pub code: String,
    pub message: String,
}

/// Ban/timeout response
#[derive(Debug, Clone, Deserialize)]
pub struct BanResponse {
    #[allow(dead_code)] // Part of Twitch API response
    pub data: Vec<BanData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BanData {
    #[allow(dead_code)] // Part of Twitch API response
    pub broadcaster_id: String,
    #[allow(dead_code)] // Part of Twitch API response
    pub moderator_id: String,
    #[allow(dead_code)] // Part of Twitch API response
    pub user_id: String,
    #[allow(dead_code)] // Part of Twitch API response
    pub created_at: String,
    #[allow(dead_code)] // Part of Twitch API response
    pub end_time: Option<String>,
}

/// User info response
#[derive(Debug, Clone, Deserialize)]
pub struct UsersResponse {
    pub data: Vec<UserData>,
}

/// User data from Twitch Helix API
#[derive(Debug, Clone, Deserialize)]
pub struct UserData {
    /// User's ID
    pub id: String,
    /// User's login name
    #[allow(dead_code)] // Part of Twitch API response
    pub login: String,
    /// User's display name
    #[allow(dead_code)] // Part of Twitch API response
    pub display_name: String,
    /// User's broadcaster type (e.g., "partner", "affiliate", or empty string)
    #[allow(dead_code)] // Part of Twitch API response
    pub broadcaster_type: String,
    /// User's channel description
    #[allow(dead_code)] // Part of Twitch API response
    pub description: String,
    /// URL to user's profile image
    #[allow(dead_code)] // Part of Twitch API response
    pub profile_image_url: String,
    /// URL to user's offline image
    #[allow(dead_code)] // Part of Twitch API response
    pub offline_image_url: String,
    /// RFC3339 timestamp of when the user account was created
    #[allow(dead_code)] // Part of Twitch API response
    pub created_at: String,
}

/// Chat settings response
#[allow(dead_code)] // Reserved for future chat settings management
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatSettingsResponse {
    pub data: Vec<ChatSettings>,
}

#[allow(dead_code)] // Reserved for future chat settings management
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatSettings {
    pub broadcaster_id: String,
    pub emote_mode: bool,
    pub follower_mode: bool,
    pub follower_mode_duration: Option<u32>,
    pub slow_mode: bool,
    pub slow_mode_wait_time: Option<u32>,
    pub subscriber_mode: bool,
    pub unique_chat_mode: bool,
}

/// Twitch API client for HTTP operations
pub struct TwitchApi {
    client: reqwest::Client,
    credentials: TwitchCredentials,
    access_token: Arc<RwLock<String>>,
    refresh_token: Arc<RwLock<String>>,
    token_refresh_tx: Option<mpsc::UnboundedSender<(String, String)>>,
    token_expired_tx: Option<mpsc::UnboundedSender<()>>,
}

impl TwitchApi {
    /// Create a new Twitch API client
    pub fn new(
        credentials: TwitchCredentials,
        access_token: String,
        refresh_token: String,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            credentials,
            access_token: Arc::new(RwLock::new(access_token)),
            refresh_token: Arc::new(RwLock::new(refresh_token)),
            token_refresh_tx: None,
            token_expired_tx: None,
        }
    }

    /// Set a channel to receive notifications when tokens are refreshed
    pub fn set_token_refresh_notifier(&mut self, tx: mpsc::UnboundedSender<(String, String)>) {
        self.token_refresh_tx = Some(tx);
    }

    /// Set a channel to receive notifications when token expires and cannot be auto-refreshed
    pub fn set_token_expired_notifier(&mut self, tx: mpsc::UnboundedSender<()>) {
        self.token_expired_tx = Some(tx);
    }

    /// Get the current access token
    pub async fn get_access_token(&self) -> String {
        self.access_token.read().await.clone()
    }

    /// Get the current refresh token
    pub async fn get_refresh_token(&self) -> String {
        self.refresh_token.read().await.clone()
    }

    /// Update tokens manually
    ///
    /// Use this when tokens are managed externally (e.g., via a web service)
    pub async fn update_tokens(&self, access_token: &str, refresh_token: &str) {
        {
            let mut token = self.access_token.write().await;
            *token = access_token.to_string();
        }
        {
            let mut token = self.refresh_token.write().await;
            *token = refresh_token.to_string();
        }
    }

    /// Refresh the access token using the refresh token
    async fn refresh_token(&self) -> Result<()> {
        let current_refresh_token = self.refresh_token.read().await.clone();

        match auth::refresh_access_token(&current_refresh_token, &self.credentials).await {
            Ok(token_response) => {
                // Update both tokens
                let new_access_token = token_response.access_token.clone();
                let new_refresh_token = token_response.refresh_token.clone();

                {
                    let mut access_token = self.access_token.write().await;
                    *access_token = new_access_token.clone();
                }
                {
                    let mut refresh_token = self.refresh_token.write().await;
                    *refresh_token = new_refresh_token.clone();
                }

                // Notify listeners that tokens were refreshed
                if let Some(tx) = &self.token_refresh_tx {
                    let _ = tx.send((new_access_token, new_refresh_token));
                }

                Ok(())
            }
            Err(e) => {
                // Check if error is due to missing client_secret
                if let TwitchError::AuthError(ref msg) = e {
                    if msg.contains("client_secret is required") {
                        // Notify that token expired and needs manual refresh
                        if let Some(tx) = &self.token_expired_tx {
                            let _ = tx.send(());
                        }
                        // Return Ok since we successfully notified about expiration
                        // The app will handle refreshing tokens externally
                        return Ok(());
                    }
                }
                Err(e)
            }
        }
    }

    /// Get user information by login name
    ///
    /// **Required OAuth Scopes**: None (public endpoint)
    pub async fn get_user_by_login(&self, login: &str) -> Result<UserData> {
        let url = format!("{}?login={}", USERS_URL, login);
        let access_token = self.access_token.read().await;

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token); // Release the lock before refreshing
                self.refresh_token().await?;
                return Box::pin(self.get_user_by_login(login)).await; // Retry with new token
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let users_response = response.json::<UsersResponse>().await?;
        users_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| TwitchError::HttpError(format!("User '{}' not found", login)))
    }

    /// Get authenticated user information
    ///
    /// **Required OAuth Scopes**: None (uses the access token to identify the user)
    pub async fn get_current_user(&self) -> Result<UserData> {
        let access_token = self.access_token.read().await;

        let response = self
            .client
            .get(USERS_URL)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.get_current_user()).await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let users_response = response.json::<UsersResponse>().await?;
        users_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| TwitchError::AuthError("Failed to get authenticated user".to_string()))
    }

    /// Send a chat message
    ///
    /// **Required OAuth Scopes**: `user:write:chat` or `user:bot`
    pub async fn send_message(
        &self,
        broadcaster_id: &str,
        sender_id: &str,
        message: &str,
    ) -> Result<SendMessageResponse> {
        let body = json!({
            "broadcaster_id": broadcaster_id,
            "sender_id": sender_id,
            "message": message
        });

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .post(CHAT_MESSAGES_URL)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.send_message(broadcaster_id, sender_id, message)).await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let send_response = response.json::<SendMessageResponse>().await?;
        Ok(send_response)
    }

    /// Reply to a chat message
    ///
    /// **Required OAuth Scopes**: `user:write:chat` or `user:bot`
    pub async fn reply_to_message(
        &self,
        broadcaster_id: &str,
        sender_id: &str,
        message: &str,
        reply_parent_message_id: &str,
    ) -> Result<SendMessageResponse> {
        let body = json!({
            "broadcaster_id": broadcaster_id,
            "sender_id": sender_id,
            "message": message,
            "reply_parent_message_id": reply_parent_message_id
        });

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .post(CHAT_MESSAGES_URL)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.reply_to_message(
                    broadcaster_id,
                    sender_id,
                    message,
                    reply_parent_message_id,
                ))
                .await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let send_response = response.json::<SendMessageResponse>().await?;
        Ok(send_response)
    }

    /// Delete a chat message
    ///
    /// **Required OAuth Scopes**: `moderator:manage:chat_messages`
    pub async fn delete_message(
        &self,
        broadcaster_id: &str,
        moderator_id: &str,
        message_id: &str,
    ) -> Result<()> {
        let url = format!(
            "{}?broadcaster_id={}&moderator_id={}&message_id={}",
            MODERATION_CHAT_URL, broadcaster_id, moderator_id, message_id
        );

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.delete_message(broadcaster_id, moderator_id, message_id))
                    .await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Ban a user permanently
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn ban_user(
        &self,
        broadcaster_id: &str,
        moderator_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<BanResponse> {
        let url = format!(
            "{}?broadcaster_id={}&moderator_id={}",
            MODERATION_BANS_URL, broadcaster_id, moderator_id
        );

        let body = json!({
            "data": {
                "user_id": user_id,
                "reason": reason
            }
        });

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.ban_user(broadcaster_id, moderator_id, user_id, reason))
                    .await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let ban_response = response.json::<BanResponse>().await?;
        Ok(ban_response)
    }

    /// Timeout a user
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn timeout_user(
        &self,
        broadcaster_id: &str,
        moderator_id: &str,
        user_id: &str,
        duration: u32,
        reason: &str,
    ) -> Result<BanResponse> {
        let url = format!(
            "{}?broadcaster_id={}&moderator_id={}",
            MODERATION_BANS_URL, broadcaster_id, moderator_id
        );

        let body = json!({
            "data": {
                "user_id": user_id,
                "duration": duration,
                "reason": reason
            }
        });

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.timeout_user(
                    broadcaster_id,
                    moderator_id,
                    user_id,
                    duration,
                    reason,
                ))
                .await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let ban_response = response.json::<BanResponse>().await?;
        Ok(ban_response)
    }

    /// Unban a user
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn unban_user(
        &self,
        broadcaster_id: &str,
        moderator_id: &str,
        user_id: &str,
    ) -> Result<()> {
        let url = format!(
            "{}?broadcaster_id={}&moderator_id={}&user_id={}",
            MODERATION_BANS_URL, broadcaster_id, moderator_id, user_id
        );

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.unban_user(broadcaster_id, moderator_id, user_id)).await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Get chat settings
    ///
    /// **Required OAuth Scopes**: `moderator:read:chat_settings`
    #[allow(dead_code)] // Reserved for future chat settings management
    pub async fn get_chat_settings(
        &self,
        broadcaster_id: &str,
        moderator_id: Option<&str>,
    ) -> Result<ChatSettings> {
        let mut url = format!("{}?broadcaster_id={}", CHAT_SETTINGS_URL, broadcaster_id);
        if let Some(mod_id) = moderator_id {
            url.push_str(&format!("&moderator_id={}", mod_id));
        }

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.get_chat_settings(broadcaster_id, moderator_id)).await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let settings_response = response.json::<ChatSettingsResponse>().await?;
        settings_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| TwitchError::HttpError("No chat settings found".to_string()))
    }

    /// Update chat settings
    ///
    /// **Required OAuth Scopes**: `moderator:manage:chat_settings`
    #[allow(dead_code)] // Reserved for future chat settings management
    pub async fn update_chat_settings(
        &self,
        broadcaster_id: &str,
        moderator_id: &str,
        settings: serde_json::Value,
    ) -> Result<ChatSettings> {
        let url = format!(
            "{}?broadcaster_id={}&moderator_id={}",
            CHAT_SETTINGS_URL, broadcaster_id, moderator_id
        );

        let access_token = self.access_token.read().await;

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", *access_token))
            .header("Client-Id", &self.credentials.client_id)
            .header("Content-Type", "application/json")
            .json(&settings)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 401 {
                drop(access_token);
                self.refresh_token().await?;
                return Box::pin(self.update_chat_settings(broadcaster_id, moderator_id, settings))
                    .await;
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TwitchError::HttpError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let settings_response = response.json::<ChatSettingsResponse>().await?;
        settings_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| TwitchError::HttpError("No chat settings in response".to_string()))
    }
}
