use tokio::sync::mpsc;
use twitchy::{TwitchClient, TwitchClientEvent, TwitchConfig, TwitchEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the Twitch client
    // NOTE: For moderation you need additional OAuth scopes:
    // - moderator:manage:chat_messages (to delete messages)
    // - moderator:manage:banned_users (to ban/timeout users)
    let config = TwitchConfig::builder()
        .channel("your_channel_name")
        .tokens("your_access_token", "your_refresh_token")
        .credentials("your_client_id", "your_client_secret")
        .build()?;

    let (tx, mut rx) = mpsc::channel(100);
    let mut client = TwitchClient::new(config);

    println!("Moderation bot starting...");
    client.connect(tx).await?;

    // List of banned words (example)
    let banned_words = vec!["spam", "scam", "badword"];

    // Maximum ratio of uppercase characters (spam protection)
    const MAX_CAPS_RATIO: f32 = 0.8; // 80% uppercase = suspicious

    while let Some(event) = rx.recv().await {
        match event {
            TwitchClientEvent::Connected => {
                println!("Bot connected and ready for moderation!");

                // Optional: send a greeting message
                if let Err(e) = client.send_message("Moderation bot is now active!").await {
                    eprintln!("Error sending message: {}", e);
                }
            }

            TwitchClientEvent::ChatEvent(TwitchEvent::ChatMessage(msg)) => {
                println!("[{}]: {}", msg.chatter_user_name, msg.message.text);

                let message_text = msg.message.text.to_lowercase();
                let user_name = &msg.chatter_user_name;
                let user_id = &msg.chatter_user_id;
                let message_id = &msg.message_id;

                // === EXAMPLE 1: Banned word filter ===
                for banned_word in &banned_words {
                    if message_text.contains(banned_word) {
                        println!(
                            "WARNING: Detected banned word '{}' from {}",
                            banned_word, user_name
                        );

                        // Delete the message
                        match client.delete_message(message_id).await {
                            Ok(_) => println!("Message deleted"),
                            Err(e) => eprintln!("Error deleting message: {}", e),
                        }

                        // Give a 60 second timeout
                        match client
                            .timeout_user(
                                user_id,
                                60,
                                &format!("Used banned word: {}", banned_word),
                            )
                            .await
                        {
                            Ok(_) => println!("Timeout 60s for {}", user_name),
                            Err(e) => eprintln!("Error timing out user: {}", e),
                        }

                        // Send warning to chat
                        let warning = format!("@{} Using banned words is not allowed!", user_name);
                        let _ = client.send_message(&warning).await;

                        break;
                    }
                }

                // === EXAMPLE 2: Excessive caps spam ===
                let caps_count = msg
                    .message
                    .text
                    .chars()
                    .filter(|c| c.is_uppercase())
                    .count();
                let total_chars = msg
                    .message
                    .text
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .count();

                if total_chars > 10 {
                    let caps_ratio = caps_count as f32 / total_chars as f32;

                    if caps_ratio > MAX_CAPS_RATIO {
                        println!(
                            "WARNING: Too many uppercase letters ({:.0}%) from {}",
                            caps_ratio * 100.0,
                            user_name
                        );

                        // Delete the message
                        let _ = client.delete_message(message_id).await;

                        // Send warning
                        let warning = format!("@{} Please don't write in all caps!", user_name);
                        let _ = client.send_message(&warning).await;
                    }
                }

                // === EXAMPLE 3: Moderator commands ===
                if message_text.starts_with("!ban") && is_moderator(&msg.badges) {
                    // Example: !ban username reason
                    let parts: Vec<&str> = msg.message.text.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let target_username = parts[1];
                        let reason = parts.get(2..).map(|p| p.join(" ")).unwrap_or_default();

                        println!(
                            "Moderator {} banning {} with reason {}",
                            user_name, target_username, reason
                        );

                        // NOTE: Here you need the user ID, not the username
                        // In a real bot you would need an API call to get user_id from username
                        // client.ban_user(&target_user_id, &reason).await?;

                        let response = format!("Banned user {}", target_username);
                        let _ = client.send_message(&response).await;
                    }
                }

                if message_text.starts_with("!timeout") && is_moderator(&msg.badges) {
                    // Example: !timeout username 600 reason
                    let parts: Vec<&str> = msg.message.text.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let target_username = parts[1];
                        let duration: u32 = parts[2].parse().unwrap_or(600);
                        let reason = parts.get(3..).map(|p| p.join(" ")).unwrap_or_default();

                        println!(
                            "Moderator {} timing out {} for {}s for reason {}",
                            user_name, target_username, duration, reason
                        );

                        // Same as above - you need user_id
                        // client.timeout_user(&target_user_id, duration, &reason).await?;

                        let response = format!("Timeout {}s for {}", duration, target_username);
                        let _ = client.send_message(&response).await;
                    }
                }

                // === EXAMPLE 4: Delete command ===
                if message_text.starts_with("!delete") && is_moderator(&msg.badges) {
                    // This command would delete a previous message (needs message_id tracking)
                    println!("Moderator {} deleting message", user_name);
                    let _ = client.send_message("Message deleted by moderator").await;
                }
            }

            TwitchClientEvent::ChatEvent(TwitchEvent::ChannelBan(ban)) => {
                println!("{} was banned", ban.user_name);
                println!("   Reason: {}", ban.reason);
            }

            TwitchClientEvent::ChatEvent(TwitchEvent::ChannelUnban(unban)) => {
                println!("{} was unbanned", unban.user_name);
            }

            TwitchClientEvent::TokensRefreshed(access, refresh) => {
                println!("Tokens refreshed - save them!");
                println!("Access:  {}", access);
                println!("Refresh: {}", refresh);
            }

            TwitchClientEvent::Disconnected => {
                println!("Bot disconnected");
            }

            _ => {}
        }
    }

    Ok(())
}

// Helper function to check if a user is a moderator
fn is_moderator(badges: &[twitchy::Badge]) -> bool {
    badges
        .iter()
        .any(|badge| badge.set_id == "moderator" || badge.set_id == "broadcaster")
}
