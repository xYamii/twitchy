use tokio::sync::mpsc;
use twitchy::{TwitchClient, TwitchClientEvent, TwitchConfig, TwitchEvent};

/// Example showing how to use twitchy with external token management
///
/// This pattern is useful when:
/// - You want to share your client_id with users without exposing client_secret
/// - Users get tokens from your OAuth service/website
/// - Tokens are refreshed externally (via your API or website)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client with only client_id (no client_secret)
    // Tokens will not be automatically refreshed
    let config = TwitchConfig::builder()
        .channel("your_channel_name")
        .tokens("your_access_token", "your_refresh_token")
        .client_id_only("your_client_id") // No client_secret needed
        .build()?;

    let (tx, mut rx) = mpsc::channel(100);
    let mut client = TwitchClient::new(config);

    println!("Connecting to Twitch with external token management...");
    client.connect(tx).await?;

    // Listen for events
    while let Some(event) = rx.recv().await {
        match event {
            TwitchClientEvent::Connected => {
                println!("Connected to Twitch!");
                println!("Tokens are managed externally - no automatic refresh.\n");
            }

            TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
                println!("[{}]: {}", msg.chatter_user_name, msg.message.text);
            }

            // IMPORTANT: Handle token expiration
            TwitchClientEvent::TokenExpired => {
                println!("\nToken expired! Manual refresh required.");
                println!("Steps to refresh:");
                println!("1. Call your API endpoint or visit your website");
                println!("2. Get new access_token and refresh_token");
                println!("3. Update the client with new tokens\n");

                // Example: In a real application, you would call your API here
                // let new_tokens = fetch_new_tokens_from_your_api().await?;
                // client.update_tokens(&new_tokens.access, &new_tokens.refresh).await;
                // println!("Tokens updated successfully!");

                // For this example, we'll just print the instructions
                println!("Simulating token refresh...");
                let new_access_token = "new_access_token_from_your_service";
                let new_refresh_token = "new_refresh_token_from_your_service";

                // Update tokens in the client
                client
                    .update_tokens(new_access_token, new_refresh_token)
                    .await;
                println!("Tokens updated! Bot continues running.\n");
            }

            TwitchClientEvent::Disconnected => {
                println!("Disconnected from Twitch");
            }

            TwitchClientEvent::Error(err) => {
                eprintln!("Error: {}", err);
            }

            _ => {}
        }
    }

    Ok(())
}

// Example function showing how you might fetch new tokens from your service
#[allow(dead_code)]
async fn fetch_new_tokens_from_your_api(
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // In a real application, you would make an HTTP request to your token refresh service
    // Example:
    //
    // let response = reqwest::Client::new()
    //     .post("https://your-api.com/refresh-tokens")
    //     .header("Authorization", "Bearer user_session_token")
    //     .send()
    //     .await?;
    //
    // let tokens: TokenResponse = response.json().await?;
    // Ok((tokens.access_token, tokens.refresh_token))

    // For this example, just return dummy tokens
    Ok((
        "new_access_token".to_string(),
        "new_refresh_token".to_string(),
    ))
}
