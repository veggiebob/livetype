use uuid::Uuid;

pub fn make_user_id(uid: String) -> UserId {
    UserId(SimpleUserId(uid))
}

pub fn make_group_chat_id() -> GroupChatId {
    GroupChatId(Uuid::new_v4())
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct SimpleUserId(String);

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct UserId(SimpleUserId);

pub type UserPair = (UserId, UserId);

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct GroupChatId(Uuid); // name: clique?

impl UserId {
    pub fn to_string(&self) -> String {
        self.0.0.clone()
    }
}