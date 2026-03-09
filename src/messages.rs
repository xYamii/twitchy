use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Helper function to set the current timestamp when deserializing
fn now() -> DateTime<Utc> {
    Utc::now()
}

/// WebSocket message received from Twitch EventSub
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventSubMessage {
    /// Metadata included in every EventSub message
    pub metadata: Metadata,
    /// Payload containing the actual message data
    pub payload: Payload,
}

/// Metadata included in every EventSub message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    /// Unique message identifier
    pub message_id: String,
    /// Type of message (welcome, notification, keepalive, reconnect, revocation)
    pub message_type: String,
    /// RFC3339 timestamp of when the message was sent
    pub message_timestamp: String,
    /// Type of subscription (only present for notifications and revocations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_type: Option<String>,
    /// Version of the subscription (only present for notifications and revocations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_version: Option<String>,
}

/// Payload of an EventSub message
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Payload {
    /// Welcome message payload sent when first connecting
    Welcome(WelcomePayload),
    /// Notification payload containing events
    Notification(NotificationPayload),
    /// Reconnect payload requesting client to reconnect
    Reconnect(ReconnectPayload),
    /// Keepalive payload to maintain connection
    Keepalive(KeepalivePayload),
    /// Revocation payload indicating subscription was revoked
    Revocation(RevocationPayload),
}

/// Session welcome message payload - sent when first connecting
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WelcomePayload {
    /// WebSocket session information
    pub session: Session,
}

/// WebSocket session information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Session {
    /// Session ID used to identify this WebSocket session
    pub id: String,
    /// Status of the session (e.g., "connected")
    pub status: String,
    /// Maximum time in seconds to wait between keepalive messages
    pub keepalive_timeout_seconds: u64,
    /// URL to reconnect to (if server requests reconnection)
    pub reconnect_url: Option<String>,
    /// RFC3339 timestamp of when the connection was established
    pub connected_at: String,
}

/// Notification payload containing events
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationPayload {
    /// Information about the subscription that triggered this notification
    pub subscription: Subscription,
    /// The actual event data (type depends on subscription type)
    pub event: serde_json::Value,
}

/// EventSub subscription information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subscription {
    /// Subscription ID
    pub id: String,
    /// Type of subscription (e.g., "channel.chat.message")
    #[serde(rename = "type")]
    pub subscription_type: String,
    /// Version of the subscription
    pub version: String,
    /// Status of the subscription (e.g., "enabled")
    pub status: String,
    /// Cost of the subscription (used for rate limiting)
    pub cost: u32,
    /// Subscription condition parameters
    pub condition: serde_json::Value,
    /// RFC3339 timestamp of when the subscription was created
    pub created_at: String,
}

/// Reconnect payload - server requests client to reconnect
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReconnectPayload {
    /// WebSocket session information with reconnect URL
    pub session: Session,
}

/// Keepalive payload (empty message to maintain connection)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeepalivePayload {}

/// Revocation payload - subscription was revoked
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevocationPayload {
    /// Information about the subscription that was revoked
    pub subscription: Subscription,
}

/// Chat message event from channel.chat.message subscription
///
/// **Required OAuth Scopes**: `user:read:chat` or `user:bot`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessageEvent {
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// Chatter's user ID
    pub chatter_user_id: String,
    /// Chatter's login name
    pub chatter_user_login: String,
    /// Chatter's display name
    pub chatter_user_name: String,
    /// Unique message ID
    pub message_id: String,
    /// The message content
    pub message: Message,
    /// Chatter's color in hex format (e.g., "#FF0000")
    pub color: String,
    /// List of chat badges the chatter has
    pub badges: Vec<Badge>,
    /// Type of message (e.g., "text", "channel_points_highlighted")
    pub message_type: String,
    /// Cheer information (if message contains bits)
    pub cheer: Option<Cheer>,
    /// Reply information (if message is a reply)
    pub reply: Option<Reply>,
    /// Channel points reward ID (if message triggered a reward)
    pub channel_points_custom_reward_id: Option<String>,
    /// RFC3339 timestamp of when this library received the message (not from Twitch API)
    #[serde(skip_deserializing, default = "now")]
    pub received_at: DateTime<Utc>,
}

impl ChatMessageEvent {
    /// Get the plain text content of the message
    pub fn text(&self) -> &str {
        &self.message.text
    }
}

/// Message content and fragments
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    /// Full message text
    pub text: String,
    /// Message broken down into fragments (text, emotes, cheermotes, mentions)
    pub fragments: Vec<MessageFragment>,
}

/// Fragment of a message (can be text, emote, cheermote, or mention)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageFragment {
    /// Type of fragment ("text", "emote", "cheermote", "mention")
    #[serde(rename = "type")]
    pub fragment_type: String,
    /// Text content of the fragment
    pub text: String,
    /// Cheermote information (if fragment is a cheermote)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cheermote: Option<Cheermote>,
    /// Emote information (if fragment is an emote)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emote: Option<Emote>,
    /// Mention information (if fragment is a mention)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention: Option<Mention>,
}

/// Cheermote information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cheermote {
    /// Cheermote prefix (e.g., "Cheer")
    pub prefix: String,
    /// Number of bits
    pub bits: u32,
    /// Cheermote tier
    pub tier: u32,
}

/// Emote information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Emote {
    /// Emote ID
    pub id: String,
    /// Emote set ID
    pub emote_set_id: String,
}

/// User mention information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mention {
    /// Mentioned user's ID
    pub user_id: String,
    /// Mentioned user's display name
    pub user_name: String,
    /// Mentioned user's login name
    pub user_login: String,
}

/// Chat badge (subscriber, moderator, VIP, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Badge {
    /// Badge set ID (e.g., "subscriber", "moderator")
    pub set_id: String,
    /// Badge ID within the set
    pub id: String,
    /// Additional badge information
    pub info: String,
}

/// Cheer (bits) information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cheer {
    /// Number of bits cheered
    pub bits: u32,
}

/// Reply information (when a message is a reply to another message)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reply {
    /// ID of the parent message being replied to
    pub parent_message_id: String,
    /// Text content of the parent message
    pub parent_message_body: String,
    /// User ID of the parent message author
    pub parent_user_id: String,
    /// Display name of the parent message author
    pub parent_user_name: String,
    /// Login name of the parent message author
    pub parent_user_login: String,
    /// ID of the root message in the thread
    pub thread_message_id: String,
    /// User ID of the thread starter
    pub thread_user_id: String,
    /// Display name of the thread starter
    pub thread_user_name: String,
    /// Login name of the thread starter
    pub thread_user_login: String,
}

/// Message delete event from channel.chat.message_delete subscription
///
/// **Required OAuth Scopes**: `user:read:chat` or `user:bot`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageDeleteEvent {
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// User ID of the message author
    pub target_user_id: String,
    /// Login name of the message author
    pub target_user_login: String,
    /// Display name of the message author
    pub target_user_name: String,
    /// ID of the deleted message
    pub message_id: String,
}

/// Clear user messages event (ban/timeout) from channel.chat.clear_user_messages subscription
///
/// **Required OAuth Scopes**: `user:read:chat` or `user:bot`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClearUserMessagesEvent {
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// User ID of the affected user
    pub target_user_id: String,
    /// Login name of the affected user
    pub target_user_login: String,
    /// Display name of the affected user
    pub target_user_name: String,
}

/// Chat clear event (entire chat cleared) from channel.chat.clear subscription
///
/// **Required OAuth Scopes**: `user:read:chat` or `user:bot`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatClearEvent {
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
}

/// Chat settings update event from channel.chat_settings.update subscription
///
/// **Required OAuth Scopes**: `user:read:chat` or `user:bot`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatSettingsUpdateEvent {
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// Whether emote-only mode is enabled
    pub emote_mode: bool,
    /// Whether follower-only mode is enabled
    pub follower_mode: bool,
    /// Minimum follow duration in minutes (if follower mode is enabled)
    pub follower_mode_duration_minutes: Option<u32>,
    /// Whether slow mode is enabled
    pub slow_mode: bool,
    /// Slow mode wait time in seconds (if slow mode is enabled)
    pub slow_mode_wait_time_seconds: Option<u32>,
    /// Whether subscriber-only mode is enabled
    pub subscriber_mode: bool,
    /// Whether unique chat mode is enabled (users must post unique messages)
    pub unique_chat_mode: bool,
}

/// Channel ban event from channel.ban subscription
///
/// **Required OAuth Scopes**: `channel:moderate` or `moderator:manage:banned_users`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelBanEvent {
    /// Banned user's ID
    pub user_id: String,
    /// Banned user's login name
    pub user_login: String,
    /// Banned user's display name
    pub user_name: String,
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// Moderator's user ID
    pub moderator_user_id: String,
    /// Moderator's login name
    pub moderator_user_login: String,
    /// Moderator's display name
    pub moderator_user_name: String,
    /// Reason for the ban
    pub reason: String,
    /// RFC3339 timestamp of when the ban occurred
    pub banned_at: String,
    /// RFC3339 timestamp of when the ban expires (None if permanent)
    pub ends_at: Option<String>,
    /// Whether the ban is permanent
    pub is_permanent: bool,
}

/// Channel unban event from channel.unban subscription
///
/// **Required OAuth Scopes**: `channel:moderate` or `moderator:manage:banned_users`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelUnbanEvent {
    /// Unbanned user's ID
    pub user_id: String,
    /// Unbanned user's login name
    pub user_login: String,
    /// Unbanned user's display name
    pub user_name: String,
    /// Broadcaster's user ID
    pub broadcaster_user_id: String,
    /// Broadcaster's login name
    pub broadcaster_user_login: String,
    /// Broadcaster's display name
    pub broadcaster_user_name: String,
    /// Moderator's user ID
    pub moderator_user_id: String,
    /// Moderator's login name
    pub moderator_user_login: String,
    /// Moderator's display name
    pub moderator_user_name: String,
}

/// Events that can be received from Twitch EventSub
#[derive(Debug, Clone)]
pub enum TwitchEvent {
    /// A chat message was sent
    ChatMessage(ChatMessageEvent),
    /// A message was deleted
    MessageDelete(MessageDeleteEvent),
    /// A user's messages were cleared (ban/timeout)
    ClearUserMessages(ClearUserMessagesEvent),
    /// The entire chat was cleared
    ChatClear(ChatClearEvent),
    /// Chat settings were updated
    ChatSettingsUpdate(ChatSettingsUpdateEvent),
    /// A user was banned or timed out
    ChannelBan(ChannelBanEvent),
    /// A user was unbanned
    ChannelUnban(ChannelUnbanEvent),
}
