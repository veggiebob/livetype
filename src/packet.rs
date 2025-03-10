use crate::identity::{UserId, make_user_id};
use crate::message_server::RoutingInfo;
use rocket_ws::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum PacketError {
    Serde(serde_json::Error),
    WrongType(Message),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UPacket {
    sender: String,
    message: String,
    destination: String,
}

#[derive(PartialEq, Eq, Debug)]
pub struct SPacket {
    pub sender: UserId,
    pub message: String,
    pub destination: UserId,
}

pub fn make_server_packet(upacket: UPacket, sender: UserId) -> SPacket {
    let UPacket {
        message,
        destination,
        ..
    } = upacket;
    SPacket {
        sender,
        message,
        destination: make_user_id(destination),
    }
}

pub fn make_upacket(spacket: SPacket) -> UPacket {
    let SPacket {
        message,
        destination,
        sender,
    } = spacket;
    UPacket {
        sender: sender.to_string(),
        message,
        destination: destination.to_string(),
    }
}

impl RoutingInfo<UserId> for SPacket {
    fn get_to_from(&self) -> (UserId, UserId) {
        (self.destination.clone(), self.sender.clone())
    }
}

impl TryFrom<Message> for UPacket {
    type Error = PacketError;
    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Text(txt) => {
                serde_json::from_str::<UPacket>(&txt).map_err(|e| PacketError::Serde(e))
            }
            v => Err(PacketError::WrongType(v)),
        }
    }
}

impl TryFrom<UPacket> for Message {
    type Error = PacketError;
    fn try_from(value: UPacket) -> Result<Self, Self::Error> {
        serde_json::to_string(&value)
            .map(|s| Message::Text(s))
            .map_err(|e| PacketError::Serde(e))
    }
}
