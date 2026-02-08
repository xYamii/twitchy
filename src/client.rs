use crate::api::TwitchApi;
use crate::config::TwitchConfig;
use crate::error::{Result, TwitchError};
use crate::eventsub::EventSubManager;
use crate::messages::TwitchEvent;
use crate::websocket::{
    reconnect_with_backoff, ConnectionState, WebSocketHandler, WebSocketMessage,
};
use tokio::task::JoinHandle;

/// Public events from the Twitch client
#[derive(Debug, Clone)]
pub enum TwitchClientEvent {
    /// Successfully connected to Twitch
    Connected,

    /// Disconnected from Twitch
    Disconnected,

    /// Chat event received
    ChatEvent(TwitchEvent),

    /// Tokens were refreshed automatically (access_token, refresh_token)
    ///
    /// Save these tokens for future use
    TokensRefreshed(String, String),

    /// Token has expired and needs to be refreshed manually
    ///
    /// This event is emitted when no client_secret is configured.
    /// Use `TwitchClient::update_tokens()` to provide new tokens.
    TokenExpired,

    /// Warning occurred (non-fatal)
    Warning(String),

    /// Error occurred
    Error(String),
}

/// Main Twitch client that manages WebSocket connection, EventSub subscriptions, and API calls
///
/// # Example
/// ```no_run
/// use twitchy::{TwitchClient, TwitchConfig, TwitchClientEvent};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TwitchConfig::builder()
///         .channel("my_channel")
///         .tokens("access_token", "refresh_token")
///         .credentials("client_id", "client_secret")
///         .build()?;
///
///     let (tx, mut rx) = mpsc::channel(100);
///     let mut client = TwitchClient::new(config);
///     client.connect(tx).await?;
///
///     while let Some(event) = rx.recv().await {
///         match event {
///             TwitchClientEvent::Connected => println!("Connected!"),
///             TwitchClientEvent::ChatEvent(ev) => println!("Event: {:?}", ev),
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub struct TwitchClient {
    config: TwitchConfig,
    api: TwitchApi,
    eventsub: EventSubManager,
    ws_handler: Option<WebSocketHandler>,
    ws_task: Option<JoinHandle<()>>,
    broadcaster_id: Option<String>,
    bot_user_id: Option<String>,
}

impl TwitchClient {
    /// Create a new Twitch client
    pub fn new(config: TwitchConfig) -> Self {
        // Create shared token storage
        let access_token =
            std::sync::Arc::new(tokio::sync::RwLock::new(config.auth_token.clone()));
        let refresh_token =
            std::sync::Arc::new(tokio::sync::RwLock::new(config.refresh_token.clone()));

        let api = TwitchApi::new(
            config.credentials.clone(),
            config.auth_token.clone(),
            config.refresh_token.clone(),
        );
        let eventsub = EventSubManager::new(
            config.credentials.clone(),
            access_token,
            refresh_token,
        );

        Self {
            config,
            api,
            eventsub,
            ws_handler: None,
            ws_task: None,
            broadcaster_id: None,
            bot_user_id: None,
        }
    }

    /// Connect to Twitch and start receiving events
    pub async fn connect(
        &mut self,
        event_tx: tokio::sync::mpsc::Sender<TwitchClientEvent>,
    ) -> Result<()> {
        // Set up token refresh notification channel
        let (token_refresh_tx, mut token_refresh_rx) = tokio::sync::mpsc::unbounded_channel();
        self.api
            .set_token_refresh_notifier(token_refresh_tx.clone());
        self.eventsub.set_token_refresh_notifier(token_refresh_tx);

        // Set up token expired notification channel
        let (token_expired_tx, mut token_expired_rx) = tokio::sync::mpsc::unbounded_channel();
        self.api.set_token_expired_notifier(token_expired_tx);

        // Spawn task to listen for token refresh events
        let event_tx_for_tokens = event_tx.clone();
        tokio::spawn(async move {
            while let Some((access_token, refresh_token)) = token_refresh_rx.recv().await {
                let _ = event_tx_for_tokens
                    .send(TwitchClientEvent::TokensRefreshed(
                        access_token,
                        refresh_token,
                    ))
                    .await;
            }
        });

        // Spawn task to listen for token expired events
        let event_tx_for_expired = event_tx.clone();
        tokio::spawn(async move {
            while token_expired_rx.recv().await.is_some() {
                let _ = event_tx_for_expired
                    .send(TwitchClientEvent::TokenExpired)
                    .await;
            }
        });

        let broadcaster = self
            .api
            .get_user_by_login(&self.config.channel_name)
            .await?;
        let bot_user = self.api.get_current_user().await?;

        self.broadcaster_id = Some(broadcaster.id.clone());
        self.bot_user_id = Some(bot_user.id.clone());

        // Create WebSocket handler
        let ws_handler = WebSocketHandler::new();
        let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<WebSocketMessage>(100);

        // Spawn WebSocket connection task
        let ws_task = {
            let ws_tx_clone = ws_tx.clone();
            let mut ws_handler_clone = ws_handler.clone();

            tokio::spawn(async move {
                if let Err(e) = ws_handler_clone.connect(ws_tx_clone).await {
                    log::error!("WebSocket connection failed: {}", e);
                }
            })
        };

        self.ws_task = Some(ws_task);

        // Wait for session ID
        let session_id = loop {
            match ws_rx.recv().await {
                Some(WebSocketMessage::SessionId(id)) => {
                    break id;
                }
                Some(WebSocketMessage::Error(e)) => {
                    return Err(TwitchError::WebSocketError(e));
                }
                None => {
                    return Err(TwitchError::WebSocketError(
                        "WebSocket channel closed".to_string(),
                    ));
                }
                _ => {}
            }
        };

        // Create EventSub subscriptions
        log::info!("Setting up EventSub subscriptions...");
        let (success_count, failed_count, warnings) = self
            .eventsub
            .subscribe_to_all_events(&session_id, &broadcaster.id, &bot_user.id)
            .await?;

        // Send warnings to UI (missing scopes)
        let has_warnings = !warnings.is_empty();
        for warning in warnings {
            let _ = event_tx.send(TwitchClientEvent::Warning(warning)).await;
        }

        // Small delay to ensure warnings are processed
        if has_warnings {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Always send subscription status to UI
        let status_msg = if success_count == 0 {
            // All failed - don't send duplicate message, warnings already sent
            None
        } else if failed_count == 0 {
            Some(format!(
                "✓ All {} EventSub subscriptions active",
                success_count
            ))
        } else {
            Some(format!(
                "⚠ EventSub: {} active, {} skipped (missing OAuth scopes)",
                success_count, failed_count
            ))
        };

        if let Some(msg) = status_msg {
            let _ = event_tx.send(TwitchClientEvent::Warning(msg)).await;
        }

        log::info!("EventSub setup complete - bot is ready");
        let _ = event_tx.send(TwitchClientEvent::Connected).await;

        // Spawn event processing task with reconnection handling
        let event_tx_clone = event_tx.clone();
        let ws_tx_clone = ws_tx.clone();
        let mut reconnect_handler = ws_handler.clone();

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                match msg {
                    WebSocketMessage::Event(event) => {
                        let _ = event_tx_clone
                            .send(TwitchClientEvent::ChatEvent(event))
                            .await;
                    }
                    WebSocketMessage::Disconnected => {
                        let _ = event_tx_clone.send(TwitchClientEvent::Disconnected).await;

                        // Attempt to reconnect with exponential backoff
                        match reconnect_with_backoff(&mut reconnect_handler, ws_tx_clone.clone(), 5)
                            .await
                        {
                            Ok(_) => {
                                let _ = event_tx_clone.send(TwitchClientEvent::Connected).await;
                            }
                            Err(e) => {
                                let _ = event_tx_clone
                                    .send(TwitchClientEvent::Error(format!(
                                        "Failed to reconnect: {}",
                                        e
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }
                    WebSocketMessage::Error(e) => {
                        let _ = event_tx_clone.send(TwitchClientEvent::Error(e)).await;
                    }
                    WebSocketMessage::Reconnect(url) => {
                        reconnect_handler.set_url(url.clone());

                        // Immediately reconnect to the new URL
                        let ws_tx_reconnect = ws_tx_clone.clone();
                        let mut handler_for_reconnect = reconnect_handler.clone();
                        handler_for_reconnect.set_url(url);

                        tokio::spawn(async move {
                            if let Err(e) = handler_for_reconnect.connect(ws_tx_reconnect).await {
                                log::error!("Failed to reconnect to new URL: {}", e);
                            }
                        });
                    }
                    _ => {}
                }
            }
        });

        // Spawn keepalive monitoring task
        let ws_handler_for_keepalive = ws_handler.clone();
        let event_tx_keepalive = event_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                if ws_handler_for_keepalive.is_keepalive_timeout().await {
                    let _ = event_tx_keepalive
                        .send(TwitchClientEvent::Error(
                            "Keepalive timeout - connection stale".to_string(),
                        ))
                        .await;
                    break;
                }
            }
        });

        self.ws_handler = Some(ws_handler);

        Ok(())
    }

    /// Send a chat message
    ///
    /// **Required OAuth Scopes**: `user:write:chat` or `user:bot`
    pub async fn send_message(&self, message: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let response = self
            .api
            .send_message(broadcaster_id, bot_user_id, message)
            .await?;

        if let Some(data) = response.data.first() {
            if !data.is_sent {
                if let Some(reason) = &data.drop_reason {
                    return Err(TwitchError::HttpError(format!(
                        "Message dropped: {} - {}",
                        reason.code, reason.message
                    )));
                }
            }
        }

        Ok(())
    }

    /// Reply to a chat message
    ///
    /// **Required OAuth Scopes**: `user:write:chat` or `user:bot`
    pub async fn reply_to_message(&self, message: &str, reply_to_message_id: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let response = self
            .api
            .reply_to_message(broadcaster_id, bot_user_id, message, reply_to_message_id)
            .await?;

        if let Some(data) = response.data.first() {
            if !data.is_sent {
                if let Some(reason) = &data.drop_reason {
                    return Err(TwitchError::HttpError(format!(
                        "Reply dropped: {} - {}",
                        reason.code, reason.message
                    )));
                }
            }
        }

        Ok(())
    }

    /// Delete a chat message
    ///
    /// **Required OAuth Scopes**: `moderator:manage:chat_messages`
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        self.api
            .delete_message(broadcaster_id, bot_user_id, message_id)
            .await?;

        Ok(())
    }

    /// Ban a user permanently
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn ban_user(&self, user_id: &str, reason: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        self.api
            .ban_user(broadcaster_id, bot_user_id, user_id, reason)
            .await?;

        Ok(())
    }

    /// Timeout a user
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn timeout_user(&self, user_id: &str, duration: u32, reason: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        self.api
            .timeout_user(broadcaster_id, bot_user_id, user_id, duration, reason)
            .await?;

        Ok(())
    }

    /// Unban a user
    ///
    /// **Required OAuth Scopes**: `moderator:manage:banned_users`
    pub async fn unban_user(&self, user_id: &str) -> Result<()> {
        let broadcaster_id = self
            .broadcaster_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        let bot_user_id = self
            .bot_user_id
            .as_ref()
            .ok_or_else(|| TwitchError::ConfigError("Not connected".to_string()))?;

        self.api
            .unban_user(broadcaster_id, bot_user_id, user_id)
            .await?;

        Ok(())
    }

    /// Get the current access token (may have been refreshed)
    pub async fn get_access_token(&self) -> String {
        self.api.get_access_token().await
    }

    /// Get the current refresh token (may have been refreshed)
    pub async fn get_refresh_token(&self) -> String {
        self.api.get_refresh_token().await
    }

    /// Get both current tokens (useful for persisting to config after refresh)
    pub async fn get_tokens(&self) -> (String, String) {
        let access_token = self.api.get_access_token().await;
        let refresh_token = self.api.get_refresh_token().await;
        (access_token, refresh_token)
    }

    /// Update tokens manually
    ///
    /// Use this method when `client_secret` is not configured and you need to
    /// provide new tokens after receiving a `TokenExpired` event.
    ///
    /// # Arguments
    /// * `access_token` - New access token
    /// * `refresh_token` - New refresh token
    ///
    /// # Example
    /// ```no_run
    /// # use twitchy::TwitchClient;
    /// # async fn example(client: &TwitchClient) {
    /// // After receiving TokenExpired event, get new tokens from your service
    /// let new_access = "new_access_token";
    /// let new_refresh = "new_refresh_token";
    /// client.update_tokens(new_access, new_refresh).await;
    /// # }
    /// ```
    pub async fn update_tokens(&self, access_token: &str, refresh_token: &str) {
        self.api.update_tokens(access_token, refresh_token).await;
    }

    /// Get the current connection state
    pub fn is_connected(&self) -> bool {
        self.ws_handler
            .as_ref()
            .map(|h| matches!(h.state(), ConnectionState::Connected))
            .unwrap_or(false)
    }

    /// Disconnect from Twitch
    pub async fn disconnect(&mut self) {
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        self.ws_handler = None;
        self.broadcaster_id = None;
        self.bot_user_id = None;
    }

    /// Get the broadcaster user ID (if connected)
    pub fn broadcaster_user_id(&self) -> Option<&String> {
        self.broadcaster_id.as_ref()
    }

    /// Get the bot user ID (if connected)
    pub fn bot_user_id(&self) -> Option<&String> {
        self.bot_user_id.as_ref()
    }

    /// Get a reference to the API client
    pub fn api(&self) -> &TwitchApi {
        &self.api
    }
}

impl Drop for TwitchClient {
    fn drop(&mut self) {
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
    }
}
