# Twitchy 🤖

An async Rust library for building Twitch chat bots using EventSub WebSocket and Helix API.

## Features

- ✅ **EventSub WebSocket** - Real-time chat events using Twitch's EventSub WebSocket
- ✅ **Helix API** - Send messages, moderate chat, and manage users
- ✅ **Flexible Token Management** - Supports automatic refresh OR manual/external token management (PKCE flow)
- ✅ **Reconnection Handling** - Automatic reconnection with exponential backoff
- ✅ **Type-Safe Events** - Strongly typed event structures for all Twitch events
- ✅ **Configurable** - Builder pattern for easy configuration

## Quick Start

```rust
use twitchy::{TwitchClient, TwitchConfig, TwitchClientEvent, TwitchEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client
    let config = TwitchConfig::builder()
        .channel("my_channel")
        .tokens("access_token", "refresh_token")
        .credentials("client_id", "client_secret")
        .build()?;

    // Create client and event channel
    let (tx, mut rx) = mpsc::channel(100);
    let mut client = TwitchClient::new(config);

    // Connect to Twitch
    client.connect(tx).await?;

    // Listen for events
    while let Some(event) = rx.recv().await {
        match event {
            TwitchClientEvent::Connected => {
                println!("Connected to Twitch!");
            }
            TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
                println!("[{}]: {}", msg.chatter_user_name, msg.message.text);

                // Reply to the message
                client.send_message("Hello from twitchy!").await?;
            }
            TwitchClientEvent::TokensRefreshed(access, refresh) => {
                println!("Tokens refreshed! Save these:");
                println!("Access: {}", access);
                println!("Refresh: {}", refresh);
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Required OAuth Scopes

The library requires different OAuth scopes depending on which features you use:

### Basic Chat (Required)
- `user:read:chat` or `user:bot` - Read chat messages
- `user:write:chat` or `user:bot` - Send chat messages

### Moderation (Optional)
- `moderator:manage:chat_messages` - Delete messages
- `moderator:manage:banned_users` - Ban/timeout/unban users
- `moderator:read:chat_settings` - Read chat settings
- `moderator:manage:chat_settings` - Update chat settings

## Configuration

The library supports two authentication modes:

### Option 1: Full Credentials (Automatic Token Refresh)

Provide both `client_id` and `client_secret` for automatic token refresh:

```rust
let config = TwitchConfig::builder()
    .channel("your_channel_name")
    .tokens("your_access_token", "your_refresh_token")
    .credentials("your_client_id", "your_client_secret")
    .build()?;
```

When tokens expire, the library automatically refreshes them and emits a `TokensRefreshed` event with the new tokens. Save these for future use.

### Option 2: PKCE Flow / External Token Management (Client ID Only)

Provide only `client_id` when using PKCE OAuth flow or external token management:

```rust
let config = TwitchConfig::builder()
    .channel("your_channel_name")
    .tokens("your_access_token", "your_refresh_token")
    .client_id_only("your_client_id") // No client_secret needed!
    .build()?;
```

**Perfect for:**
- Using web-based token generators with PKCE flow
- Distributing your app without exposing `client_secret`
- Managing tokens through an external service/API

When tokens expire, the library emits a `TokenExpired` event. Refresh tokens through your service and update them:

```rust
// In your event loop
TwitchClientEvent::TokenExpired => {
    // Get new tokens from your token service/API
    let (new_access, new_refresh) = fetch_tokens_from_your_service().await?;

    // Update the client
    client.update_tokens(&new_access, &new_refresh).await;
}
```

## Event Types

The library supports the following Twitch events:

- `ChatMessage` - A message was sent in chat
- `MessageDelete` - A message was deleted
- `ClearUserMessages` - A user's messages were cleared (ban/timeout)
- `ChatClear` - The entire chat was cleared
- `ChatSettingsUpdate` - Chat settings were updated
- `ChannelBan` - A user was banned or timed out
- `ChannelUnban` - A user was unbanned

## API Methods

### Sending Messages
```rust
// Send a message
client.send_message("Hello, chat!").await?;

// Reply to a message
client.reply_to_message("Thanks!", message_id).await?;
```

### Moderation
```rust
// Delete a message
client.delete_message(message_id).await?;

// Ban a user
client.ban_user(user_id, "Reason").await?;

// Timeout a user
client.timeout_user(user_id, 600, "Reason").await?;

// Unban a user
client.unban_user(user_id).await?;
```

## Examples

See the `examples/` directory for more complete examples:

- `basic_chat.rs` - Simple bot that connects and prints chat messages
- `moderation_bot.rs` - Bot with automatic spam detection and mod commands
- `external_token_management.rs` - Using the library with external token management (no client_secret)

## Architecture

The library is built around three main components:

1. **TwitchClient** - Main interface for connecting and interacting with Twitch
2. **TwitchApi** - HTTP client for Helix API operations (send messages, moderation, etc.)
3. **EventSubManager** - Manages EventSub subscriptions via WebSocket

Events are delivered through a `tokio::sync::mpsc::channel`, making it easy to integrate with async Rust applications.

## License

soon ™