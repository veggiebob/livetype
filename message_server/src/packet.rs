use std::time::SystemTime;
use crate::identity::{make_user_id, UserId};
use rocket_ws::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use uuid;
use crate::protocol::{MessageId, Timestamp};
// ------------------------- Web Packets -----------------------------

/// This is what is sent on the websocket
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct WebPacket {
    content: Packet,
    destination: WebDest,
    sender: Option<String>, // only used going toward client
    timestamp: Option<Timestamp>, // only used going toward client
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum WebDest {
    User(String),
    // Group(Uuid) // sometime later for group chats
}

// ----------------------------- Common Usage ----------------------------

/// Packet Message
#[derive(Deserialize, Serialize, Eq, PartialEq, Debug)]
pub enum Packet {
    ///
    NewMessage {
        #[serde(with = "uuid::serde::compact")]
        uuid: Uuid,
        content: String,
        start_time: Timestamp,
        end_time: Timestamp
    },
    // SyncMessage(Uuid, String), // to be used to sync database w/ chats
    /// A user only has one draft at a time in a conversation - the last thing they typed
    StartDraft,
    /// Sent back to the sender after starting a new draft
    NewDraft {
        #[serde(with = "uuid::serde::compact")]
        uuid: MessageId,
    },
    EndDraft {
        #[serde(with = "uuid::serde::compact")]
        uuid: MessageId,
        content: Option<String>, // just to sync easier
    },
    DiscardDraft {
        #[serde(with = "uuid::serde::compact")]
        uuid: MessageId
    },
    Edit {
        #[serde(with = "uuid::serde::compact")]
        uuid: MessageId,
        content: String,
    },
}

// ----------------------- Server Packets -------------------------

/// Correctly annotated & authenticated packet
#[derive(PartialEq, Eq, Debug)]
pub struct SPacket {
    pub sender: UserId,
    pub destination: Destination,
    pub time: Timestamp,
    pub packet: Packet,
}

/// Determines how to route server packet
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub enum Destination {
    User(UserId),
    // Group(...?) // future use
}

#[derive(Debug)]
pub enum PacketError {
    Serde(serde_json::Error),
    WrongType(Message),
}

pub fn get_current_time() -> Timestamp {
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    now.as_micros() as Timestamp
}

pub fn make_uuid() -> Uuid {
    Uuid::new_v4()
}

pub fn make_server_packet(webpacket: WebPacket, sender: UserId) -> SPacket {
    let destination = match &webpacket.destination {
        WebDest::User(uid) => Destination::User(make_user_id(uid.clone()))
    };
    // we ignore webpacket.sender and webpacket.timestamp
    // because that's only for sending it back
    SPacket {
        sender,
        time: get_current_time(),
        destination,
        packet: webpacket.content,
    }
}

pub fn make_webpacket(spacket: SPacket) -> WebPacket {
    let destination = match spacket.destination {
        Destination::User(uid) => WebDest::User(uid.to_string())
    };
    WebPacket {
        destination,
        sender: Some(spacket.sender.to_string()),
        timestamp: Some(spacket.time),
        content: spacket.packet,
    }
}
pub trait RoutingInfo {
    fn get_to_from(&self) -> (Destination, UserId);
}

impl RoutingInfo for SPacket {
    fn get_to_from(&self) -> (Destination, UserId) {
        (self.destination.clone(), self.sender.clone())
    }
}

impl TryFrom<Message> for WebPacket {
    type Error = PacketError;
    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Text(txt) => {
                serde_json::from_str::<WebPacket>(&txt).map_err(|e| PacketError::Serde(e))
            }
            v => Err(PacketError::WrongType(v)),
        }
    }
}

impl TryFrom<WebPacket> for Message {
    type Error = PacketError;
    fn try_from(value: WebPacket) -> Result<Self, Self::Error> {
        serde_json::to_string(&value)
            .map(|s| Message::Text(s))
            .map_err(|e| PacketError::Serde(e))
    }
}
