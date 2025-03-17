use uuid::Uuid;
use crate::identity::UserId;

/// Unix microseconds. See get_current_time()
pub type Timestamp = u64;
pub type MessageId = Uuid;

#[derive(Debug)]
pub struct Draft {
    pub id: MessageId,
    pub content: String,
    pub start_time: Timestamp,
}

/// Should be similar to frontend Message, but slightly more space conscious.
/// Also, it only exists in the context of a MessageHistory.
#[derive(Debug)]
pub struct Message {
    pub sender: UserId,
    pub content: String,
    pub id: MessageId,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
}

impl Draft {
    pub fn into_message(self, sender: UserId, time: Timestamp) -> Message {
        Message {
            sender,
            content: self.content,
            id: self.id,
            start_time: self.start_time,
            end_time: time
        }
    }
}