
// This protocol has to be synchronized with the message server backend

/*
// ------------------------- Current Web Packet Rust Spec -----------------------------
pub struct WebPacket {
    content: Packet,
    destination: WebDest,
    sender: Option<String>, // only used going toward client
    timestamp: Option<Timestamp>, // only used going toward client
}
enum WebDest {
    User(String),
    // Group(Uuid) // sometime later for group chats
}
pub enum Packet {
    ///
    NewMessage { content: String },
    // SyncMessage(Uuid, String), // to be used to sync database w/ chats
    /// A user only has one draft at a time in a conversation - the last thing they typed
    StartDraft,
    NewDraft {
        uuid: Uuid,
    },
    EndDraft {
        #[serde(with = "uuid::serde::compact")]
        uuid: Uuid,
        content: String,
    },
    Edit {
        #[serde(with = "uuid::serde::compact")]
        uuid: Uuid,
        content: String,
    },
}
*/

type Uuid = Array<number>;
type Base64Uuid = string;
type Timestamp = number;
type UserId = string;

interface WebPacket {
  content: Packet,
  destination: WebDest,
  sender?: UserId,
  timestamp?: Timestamp,
}

interface WebDest {
  User: UserId
  // Group: Uuid // sometime later for group chats
}

interface Packet {
  // all of the variants are optional, but at least one should be present
  NewMessage?: {
    uuid: Uuid,
    content: string
  },
  // SyncMessage: Uuid, String // to be used to sync database w/ chats
  StartDraft?: null,
  NewDraft?: {
    uuid: Uuid,
  },
  EndDraft?: {
    uuid: Uuid,
    content: string,
  },
  Edit?: {
    uuid: Uuid,
    content: string
  }
}


// -------------------- frontend use --------------------

// maybe this will also be how messages are stored in a database
interface Message {
  sender: UserId,
  destination: WebDest,
  content: string,
  uuid: Base64Uuid,
  start_time: Timestamp,
  end_time: Timestamp,
}

interface Draft {
  uuid?: Base64Uuid,
  content: string,
  start_time?: Timestamp,
  end_time?: Timestamp,
}

const assertUuid = (uuid: Uuid | Base64Uuid | null | undefined): Uuid | Base64Uuid => {
  if (uuid === null || uuid === undefined) {
    throw new Error('uuid is null');
  }
  return uuid;
}

const assertUserId = (userId: UserId | null | undefined): UserId => {
  if (userId === null || userId === undefined) {
    throw new Error('userId is null');
  }
  return userId;
}

// convert to base64 and back 
const uuid2str = (arr: Array<number>): Base64Uuid => btoa(String.fromCharCode(...arr));
const str2uuid = (str: Base64Uuid): Uuid => Array.from(atob(str).split('').map(c => c.charCodeAt(0)));

export type { Base64Uuid, Uuid, Timestamp, UserId, WebPacket, WebDest, Packet, Message, Draft };
export { assertUuid, assertUserId, uuid2str, str2uuid };