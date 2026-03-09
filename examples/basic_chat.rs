use tokio::sync::mpsc;
use twitchy::{TwitchClient, TwitchClientEvent, TwitchConfig, TwitchEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the Twitch client using tokens from PKCE OAuth flow
    // Generate tokens at: https://your-token-generator.example.com
    // Required scopes: user:read:chat (to read chat messages)
    //
    // Note: Using .client_id_only() means the library won't automatically refresh tokens.
    // When tokens expire, you'll receive a TokenExpired event and need to manually
    // refresh them through your token service.
    let config = TwitchConfig::builder()
        .channel("your_channel_name") // Channel name to connect to
        .tokens(
            "your_access_token",  // Your access token
            "your_refresh_token", // Your refresh token
        )
        .credentials(
            "your_client_id",     // Your application Client ID
            "your_client_secret", // Your application Client Secret
        )
        .build()?;

    // Create event channel
    let (tx, mut rx) = mpsc::channel(100);

    // Create Twitch client
    let mut client = TwitchClient::new(config);

    println!("Connecting to Twitch...");

    // Connect to Twitch
    client.connect(tx).await?;

    // Listen for events from Twitch
    while let Some(event) = rx.recv().await {
        match event {
            // Successfully connected
            TwitchClientEvent::Connected => {
                println!("Connected to Twitch!");
                println!("Waiting for chat messages...\n");
            }

            // Received a chat message
            TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
                println!(
                    "[{}]: {} at: {}",
                    msg.chatter_user_name, msg.message.text, msg.received_at
                );

                // You can also access additional message information:
                // println!("  User ID: {}", msg.chatter_user_id);
                // println!("  Message ID: {}", msg.message_id);
                // println!("  Received at: {}", msg.received_at.format("%Y-%m-%d %H:%M:%S"));
                // println!("  Color: {}", msg.color);
                // println!("  Badges: {:?}", msg.badges);
            }

            // Warning messages (missing scopes, subscription info, etc.)
            TwitchClientEvent::Warning(msg) => {
                println!("⚠ Warning: {}", msg);
            }

            // Error messages
            TwitchClientEvent::Error(err) => {
                eprintln!("❌ Error: {}", err);
            }

            // Message was deleted
            TwitchClientEvent::ChatEvent(TwitchEvent::MessageDelete(delete)) => {
                println!("Message deleted from {}", delete.target_user_name);
            }

            // Chat was cleared
            TwitchClientEvent::ChatEvent(TwitchEvent::ChatClear(_)) => {
                println!("Chat was cleared!");
            }

            // User was banned or timed out
            TwitchClientEvent::ChatEvent(TwitchEvent::ChannelBan(ban)) => {
                if let Some(ends_at) = ban.ends_at {
                    println!("Timeout for {}: until {}", ban.user_name, ends_at);
                } else {
                    println!("Permanent ban for {}", ban.user_name);
                }
                println!("   Reason: {}", ban.reason);
            }

            // User was unbanned
            TwitchClientEvent::ChatEvent(TwitchEvent::ChannelUnban(unban)) => {
                println!("{} was unbanned", unban.user_name);
            }

            // Token has expired - need to refresh manually
            TwitchClientEvent::TokenExpired => {
                println!("\n⚠ Token expired!");
                println!("Please refresh your tokens using your token service and update them.");
                println!("Then call: client.update_tokens(new_access, new_refresh).await\n");

                // Since we're using PKCE (no client secret), we can't auto-refresh.
                // You need to refresh tokens through your token service and then:
                // client.update_tokens("new_access_token", "new_refresh_token").await;
            }

            // Tokens were refreshed automatically (only if client_secret was provided)
            TwitchClientEvent::TokensRefreshed(access, refresh) => {
                println!("\nTokens refreshed! SAVE THESE:");
                println!("Access Token:  {}", access);
                println!("Refresh Token: {}\n", refresh);

                // IMPORTANT: Save these tokens for next time
            }

            // Disconnected
            TwitchClientEvent::Disconnected => {
                println!("Disconnected from Twitch");
            }

            // Other events
            _ => {}
        }
    }

    Ok(())
}
