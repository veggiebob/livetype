use crate::identity::{GroupChatId, UserId, UserPair};
use crate::packet::Destination;
use crate::protocol::{Message, MessageId};
pub mod memory_storage;

pub trait MessageFilter {
    fn include_message(message: &Message) -> bool;
}

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub enum RoomId {
    DM(UserPair),
    Group(GroupChatId),
}

#[derive(Debug)]
pub enum MessageDAOError {
    MissingMessageId(MessageId),
    MissingRoomId(RoomId),
}

pub type Result<T> = std::result::Result<T, MessageDAOError>;

// the big db interface
pub trait MessagesDAO {
    type RoomDAO: MessageRoomDAO;

    fn add_message(&mut self, message: Message, destination: Destination) -> Result<()>;

    fn get_room(&self, room_id: &RoomId) -> Result<&Self::RoomDAO>;

    fn get_room_mut(&mut self, room_id: &RoomId) -> Result<&mut Self::RoomDAO>;
}

pub trait MessageRoomDAO {
    fn new<M: Iterator<Item = UserId>>(members: M, is_dm: bool) -> Self;
    fn get_messages<F: MessageFilter>(&self, filter: &F) -> Vec<&Message>;

    fn add_message(&mut self, message: Message) -> Result<()>;

    fn get_message_mut(&mut self, m_id: MessageId) -> Option<&mut Message>;

    fn get_message(&self, m_id: MessageId) -> Option<&Message>;

    fn edit_message(&mut self, m_id: MessageId, new_content: String) -> Result<()> {
        self.get_message_mut(m_id)
            .map(|m| m.content = new_content)
            .ok_or(MessageDAOError::MissingMessageId(m_id))
    }

    // fn remove_message(&mut self, m_id: MessageId) -> Result<()>;
}

impl From<(UserId, Destination)> for RoomId {
    fn from(value: (UserId, Destination)) -> Self {
        let (user, dest) = value;
        match dest {
            Destination::User(recipient) => RoomId::DM((user, recipient)),
        }
    }
}
