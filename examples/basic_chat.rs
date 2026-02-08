use tokio::sync::mpsc;
use twitchy::{TwitchClient, TwitchClientEvent, TwitchConfig, TwitchEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the Twitch client
    // You need to provide your OAuth credentials
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
                println!("[{}]: {}", msg.chatter_user_name, msg.message.text);

                // You can also access additional message information:
                // println!("  User ID: {}", msg.chatter_user_id);
                // println!("  Message ID: {}", msg.message_id);
                // println!("  Badges: {:?}", msg.badges);
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

            // Tokens were refreshed - IMPORTANT: save them!
            TwitchClientEvent::TokensRefreshed(access, refresh) => {
                println!("\nTokens refreshed! SAVE THESE:");
                println!("Access Token:  {}", access);
                println!("Refresh Token: {}\n", refresh);

                // IMPORTANT: Here you should save the new tokens to a file or database
                // to use them on the next run
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
