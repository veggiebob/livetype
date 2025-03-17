use crate::identity::{GroupChatId, UserId, UserPair};
use crate::packet::Destination;
use crate::protocol::{Message, MessageId};
pub mod memory_storage;

pub trait MessageFilter {
    fn include_message(message: &Message) -> bool;
}

pub enum RoomId {
    DM(UserPair),
    Group(GroupChatId)
}

#[derive(Debug)]
pub enum MessageDAOError {
    MissingMessageId(MessageId),
}

pub type Result<T> = std::result::Result<T, MessageDAOError>;

// the big db interface
pub trait MessagesDAO {
    type RoomDAO: MessageRoomDAO;

    fn add_message(&mut self, message: Message, destination: Destination) -> Result<()>;

    fn get_room(&self, room_id: &RoomId) -> Result<&Self::RoomDAO>;

    fn get_room_mut(&self, room_id: &RoomId) -> Result<&mut Self::RoomDAO>;

}

pub trait MessageRoomDAO {
    fn new<M: Iterator<Item = UserId>>(members: M, is_dm: bool) -> Self;
    fn get_messages<F: MessageFilter>(&self, filter: &F) -> Vec<&Message>;

    fn edit_message(&mut self, m_id: MessageId, new_content: String) -> Result<()>;

    fn add_message(&mut self, message: Message) -> Result<()>;

    // fn remove_message(&mut self, m_id: MessageId) -> Result<()>;
}

impl From<(UserId, Destination)> for RoomId {
    fn from(value: (UserId, Destination)) -> Self {
        let (user, dest) = value;
        match dest {
            Destination::User(recipient) => RoomId::DM((user, recipient))
        }
    }
}

