//! # Twitchy
//!
//! An async Rust library for building Twitch chat bots using EventSub WebSocket and Helix API.
//!
//! ## Features
//!
//! - **EventSub WebSocket**: Real-time chat events using Twitch's EventSub WebSocket
//! - **Helix API**: Send messages, moderate chat, and manage users
//! - **Flexible Token Management**: Supports both automatic refresh (with client_secret) and manual/external token management (PKCE flow)
//! - **Reconnection Handling**: Automatic reconnection with exponential backoff
//! - **Type-Safe Events**: Strongly typed event structures for all Twitch events
//!
//! ## Quick Start
//!
//! ### Option 1: Using PKCE Flow (No Client Secret Required)
//!
//! This is ideal when you want users to authenticate through a web-based token generator
//! without exposing your client secret. Tokens must be refreshed manually or through an
//! external service.
//!
//! ```no_run
//! use twitchy::{TwitchClient, TwitchConfig, TwitchClientEvent, TwitchEvent};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure with only client_id (no client_secret)
//!     let config = TwitchConfig::builder()
//!         .channel("my_channel")
//!         .tokens("access_token", "refresh_token")
//!         .client_id_only("your_client_id") // PKCE flow - no secret needed!
//!         .build()?;
//!
//!     let (tx, mut rx) = mpsc::channel(100);
//!     let mut client = TwitchClient::new(config);
//!     client.connect(tx).await?;
//!
//!     while let Some(event) = rx.recv().await {
//!         match event {
//!             TwitchClientEvent::Connected => println!("Connected!"),
//!             TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
//!                 println!("[{}]: {}", msg.chatter_user_name, msg.message.text);
//!             }
//!             TwitchClientEvent::TokenExpired => {
//!                 // Refresh tokens through your service, then:
//!                 // client.update_tokens("new_access", "new_refresh").await;
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Option 2: Automatic Token Refresh (With Client Secret)
//!
//! ```no_run
//! use twitchy::{TwitchClient, TwitchConfig, TwitchClientEvent, TwitchEvent};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure with client_secret for automatic refresh
//!     let config = TwitchConfig::builder()
//!         .channel("my_channel")
//!         .tokens("access_token", "refresh_token")
//!         .credentials("client_id", "client_secret")
//!         .build()?;
//!
//!     let (tx, mut rx) = mpsc::channel(100);
//!     let mut client = TwitchClient::new(config);
//!     client.connect(tx).await?;
//!
//!     while let Some(event) = rx.recv().await {
//!         match event {
//!             TwitchClientEvent::Connected => println!("Connected!"),
//!             TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
//!                 println!("[{}]: {}", msg.chatter_user_name, msg.message.text);
//!                 client.send_message("Hello from twitchy!").await?;
//!             }
//!             TwitchClientEvent::TokensRefreshed(access, refresh) => {
//!                 // Tokens auto-refreshed! Save them for next time.
//!                 println!("Access: {}, Refresh: {}", access, refresh);
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Required OAuth Scopes
//!
//! The library requires different OAuth scopes depending on which features you use:
//!
//! ### Basic Chat (Required)
//! - `user:read:chat` or `user:bot` - Read chat messages
//! - `user:write:chat` or `user:bot` - Send chat messages
//!
//! ### Moderation (Optional)
//! - `moderator:manage:chat_messages` - Delete messages
//! - `moderator:manage:banned_users` - Ban/timeout/unban users
//! - `moderator:read:chat_settings` - Read chat settings
//! - `moderator:manage:chat_settings` - Update chat settings
//!
//! ## Architecture
//!
//! The library is built around three main components:
//!
//! 1. **TwitchClient**: Main interface for connecting and interacting with Twitch
//! 2. **TwitchApi**: HTTP client for Helix API operations (send messages, moderation, etc.)
//! 3. **EventSubManager**: Manages EventSub subscriptions via WebSocket
//!
//! Events are delivered through a `tokio::sync::mpsc::channel`, making it easy to integrate
//! with async Rust applications.

#![warn(missing_docs)]

mod api;
mod auth;
mod client;
mod config;
mod error;
mod eventsub;
mod messages;
mod websocket;

// Public API exports
pub use client::{TwitchClient, TwitchClientEvent};
pub use config::{TwitchConfig, TwitchConfigBuilder, TwitchCredentials};
pub use error::{Result, TwitchError};
pub use messages::{
    Badge, ChannelBanEvent, ChannelUnbanEvent, ChatClearEvent, ChatMessageEvent,
    ChatSettingsUpdateEvent, Cheer, Cheermote, ClearUserMessagesEvent, Emote, EventSubMessage,
    KeepalivePayload, Mention, Message, MessageDeleteEvent, MessageFragment, Metadata,
    NotificationPayload, Payload, ReconnectPayload, Reply, RevocationPayload, Session,
    Subscription, TwitchEvent, WelcomePayload,
};

// Re-export commonly used types for convenience
pub use api::{TwitchApi, UserData};
pub use websocket::ConnectionState;

// Re-export chrono types for timestamp handling
pub use chrono::{DateTime, Utc};
