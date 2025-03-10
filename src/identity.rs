pub fn make_user_id(uid: String) -> UserId {
    UserId(SimpleUserId(uid))
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct SimpleUserId(String);

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct UserId(SimpleUserId);

impl UserId {
    pub fn to_string(&self) -> String {
        self.0.0.clone()
    }
}