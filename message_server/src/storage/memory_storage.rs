
// implementation of message storage as in-memory :)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::hash_map::Entry;
use crate::identity::{GroupChatId, UserId, UserPair};
use crate::packet::Destination;
use crate::protocol::{Message, MessageId, Timestamp};
use crate::storage;
use crate::storage::{MessageDAOError, MessageFilter, MessageRoomDAO, MessagesDAO, RoomId};
use crate::storage::Result;

/// Contains messages in a room. Either a dm or a group chat.
pub struct MemoryMessageRoom {
    members: HashSet<UserId>,
    is_dm: bool,
    message_order: BTreeMap<Timestamp, MessageId>,
    messages: HashMap<MessageId, Message>,
}

/// Contains all messages
pub struct MemoryMessageDatabase {
    direct_messages: HashMap<UserPair, MemoryMessageRoom>,
    group_messages: HashMap<GroupChatId, MemoryMessageRoom>,
}

impl MessageRoomDAO for MemoryMessageRoom {
    fn new<M: Iterator<Item = UserId>>(members: M, is_dm: bool) -> MemoryMessageRoom {
        MemoryMessageRoom {
            members: members.collect(),
            is_dm,
            message_order: Default::default(),
            messages: Default::default(),
        }
    }

    fn get_messages<F: MessageFilter>(&self, filter: &F) -> Vec<&Message> {
        todo!()
    }

    fn add_message(&mut self, message: Message) -> storage::Result<()> {
        self.messages.insert(message.id, message);
        Ok(())
    }

    fn get_message_mut(&mut self, m_id: MessageId) -> Option<&mut Message> {
        self.messages.get_mut(&m_id)
    }

    fn get_message(&self, m_id: MessageId) -> Option<&Message> {
        self.messages.get(&m_id)
    }
}

impl MemoryMessageDatabase {
    pub fn new() -> MemoryMessageDatabase {
        MemoryMessageDatabase {
            group_messages: HashMap::new(),
            direct_messages: HashMap::new(),
        }
    }
}
impl MessagesDAO for MemoryMessageDatabase {
    type RoomDAO = MemoryMessageRoom;
    fn add_message(&mut self, message: Message, destination: Destination) -> Result<()> {
        info!("Adding message to in-memory db: {:?}", &message);
        match destination {
            Destination::User(userid) => {
                let sender = message.sender.clone();
                match self.direct_messages.entry((message.sender.clone(), userid.clone())) {
                    Entry::Occupied(mut entry) => {
                        info!("adding message to storage using existing room");
                        entry.get_mut().add_message(message)?
                    },
                    Entry::Vacant(entry) => {
                        info!("creating new message room with {:?} and {:?}", &sender, &userid);
                        entry.insert(MemoryMessageRoom::new(
                            vec![sender, userid].into_iter(),
                            true,
                        ));
                    },
                };
                Ok(())
            } // self.group_messages
        }
    }

    fn get_room(&self, room_id: &RoomId) -> Result<&MemoryMessageRoom> {
        info!("Current dms: {:?}", self.direct_messages.keys());
        match room_id {
            RoomId::DM(userpair) => self.direct_messages.get(userpair)
                .ok_or(MessageDAOError::MissingRoomId(room_id.clone())),
            RoomId::Group(gc_id) => self.group_messages.get(gc_id)
                .ok_or(MessageDAOError::MissingRoomId(room_id.clone()))
        }
    }

    fn get_room_mut(&mut self, room_id: &RoomId) -> Result<&mut MemoryMessageRoom> {
        match room_id {
            RoomId::DM(userpair) => self.direct_messages.get_mut(userpair)
                .ok_or(MessageDAOError::MissingRoomId(room_id.clone())),
            RoomId::Group(gc_id) => self.group_messages.get_mut(gc_id)
                .ok_or(MessageDAOError::MissingRoomId(room_id.clone()))
        }
    }
}