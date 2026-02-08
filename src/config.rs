use crate::error::{Result, TwitchError};

/// Twitch application credentials
#[derive(Debug, Clone)]
pub struct TwitchCredentials {
    /// Twitch application client ID
    pub client_id: String,
    /// Twitch application client secret
    pub client_secret: String,
}

impl TwitchCredentials {
    /// Create new credentials
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }
}

/// Configuration for the Twitch client
#[derive(Debug, Clone)]
pub struct TwitchConfig {
    /// The Twitch channel name to connect to
    pub channel_name: String,
    /// OAuth access token
    pub auth_token: String,
    /// OAuth refresh token
    pub refresh_token: String,
    /// Twitch application credentials
    pub credentials: TwitchCredentials,
}

impl TwitchConfig {
    /// Create a new configuration builder
    ///
    /// # Example
    /// ```no_run
    /// use twitchy::TwitchConfig;
    ///
    /// let config = TwitchConfig::builder()
    ///     .channel("my_channel")
    ///     .tokens("access_token", "refresh_token")
    ///     .credentials("client_id", "client_secret")
    ///     .build()
    ///     .expect("Failed to build config");
    /// ```
    pub fn builder() -> TwitchConfigBuilder {
        TwitchConfigBuilder::default()
    }
}

/// Builder for TwitchConfig
#[derive(Default)]
pub struct TwitchConfigBuilder {
    channel_name: Option<String>,
    auth_token: Option<String>,
    refresh_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl TwitchConfigBuilder {
    /// Set the channel name to connect to
    pub fn channel(mut self, name: impl Into<String>) -> Self {
        self.channel_name = Some(name.into());
        self
    }

    /// Set the OAuth tokens (access token and refresh token)
    pub fn tokens(
        mut self,
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Self {
        self.auth_token = Some(access_token.into());
        self.refresh_token = Some(refresh_token.into());
        self
    }

    /// Set the Twitch application credentials
    pub fn credentials(
        mut self,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        self.client_id = Some(client_id.into());
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Build the TwitchConfig
    ///
    /// # Errors
    /// Returns a `ConfigError` if any required field is missing
    pub fn build(self) -> Result<TwitchConfig> {
        let channel_name = self
            .channel_name
            .ok_or_else(|| TwitchError::ConfigError("channel_name is required".to_string()))?;

        let auth_token = self
            .auth_token
            .ok_or_else(|| TwitchError::ConfigError("auth_token is required".to_string()))?;

        let refresh_token = self
            .refresh_token
            .ok_or_else(|| TwitchError::ConfigError("refresh_token is required".to_string()))?;

        let client_id = self
            .client_id
            .ok_or_else(|| TwitchError::ConfigError("client_id is required".to_string()))?;

        let client_secret = self
            .client_secret
            .ok_or_else(|| TwitchError::ConfigError("client_secret is required".to_string()))?;

        Ok(TwitchConfig {
            channel_name,
            auth_token,
            refresh_token,
            credentials: TwitchCredentials {
                client_id,
                client_secret,
            },
        })
    }
}
