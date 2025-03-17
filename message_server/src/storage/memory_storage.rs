
// implementation of message storage as in-memory :)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::hash_map::Entry;
use crate::identity::{GroupChatId, UserId, UserPair};
use crate::packet::Destination;
use crate::protocol::{Message, MessageId, Timestamp};
use crate::storage;
use crate::storage::{MessageFilter, MessageRoomDAO, MessagesDAO, RoomId};
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

    fn edit_message(&mut self, m_id: MessageId, new_content: String) -> storage::Result<()> {
        todo!()
    }

    fn add_message(&mut self, message: Message) -> storage::Result<()> {
        self.messages.insert(message.id.clone(), message);
        Ok(())
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
                    Entry::Occupied(mut entry) => entry.get_mut().add_message(message)?,
                    Entry::Vacant(entry) => {
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
        todo!()
    }

    fn get_room_mut(&self, room_id: &RoomId) -> Result<&mut MemoryMessageRoom> {
        todo!()
    }
}