use crate::identity::UserId;
use crate::packet::{Destination, Packet, RoutingInfo, SPacket, get_current_time, make_uuid};
use crate::protocol::{Draft, Timestamp};
use crate::storage::{MessageDAOError, MessageRoomDAO, MessagesDAO, RoomId};
use rocket::fairing::{Fairing, Info};
use rocket::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rocket::{Orbit, Rocket};
use std::collections::{HashMap, VecDeque};
use std::panic;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

pub struct MessageServer<DB> {
    open_senders: HashMap<UserId, UnboundedSender<SPacket>>,
    backlog: HashMap<UserId, VecDeque<SPacket>>,
    current_drafts: HashMap<(UserId, Destination), Draft>,
    storage: DB,
}

#[derive(Debug)]
pub enum ServerError {
    AlreadyInUse(UserId),
    TrySendError(UserId),
    DAOError(MessageDAOError),
}

impl<DB: Send + 'static + MessagesDAO> MessageServer<DB> {
    pub fn new(storage: DB) -> Self {
        MessageServer {
            backlog: HashMap::new(),
            open_senders: HashMap::new(),
            current_drafts: HashMap::new(),
            storage,
        }
    }
    pub fn start(storage: DB) -> (Sender<SPacket>, Arc<Mutex<Self>>, ShutdownHandler) {
        let server = Self::new(storage);
        let server = Arc::new(Mutex::new(server));
        let server2 = Arc::clone(&server);
        let (tx, rx) = mpsc::channel();
        println!("Server started!");
        let handle = std::thread::spawn(move || {
            for spacket in rx {
                let mut s = server.lock().unwrap();
                match s.process_message(spacket) {
                    Ok(_sent) => {}
                    Err(e) => {
                        error!("Error occurred while sending message: {e:?}")
                    }
                };
            }
        });
        (tx, server2, ShutdownHandler::new(handle))
    }
    pub fn register(&mut self, uid: UserId) -> Result<UnboundedReceiver<SPacket>, ServerError> {
        // reject if this user is already connected
        if self.open_senders.contains_key(&uid) {
            return Err(ServerError::AlreadyInUse(uid));
        }
        // create channel, connect them
        let (tx, rx) = rocket::futures::channel::mpsc::unbounded();
        self.open_senders.insert(uid.clone(), tx);

        // catch them up
        self.flush_backlog(&uid)?;
        let time = get_current_time();

        for ((sender, dest), draft) in self.current_drafts.iter() {
            match dest {
                Destination::User(to) => {
                    if to == &uid {
                        info!("Catching up user {:?}", to);
                        // this is the one we just created
                        if let Some(tx) = self.open_senders.get(&uid) {
                            tx.unbounded_send(SPacket {
                                sender: sender.clone(),
                                destination: dest.clone(),
                                time,
                                packet: Packet::NewDraft {
                                    uuid: draft.id,
                                    start_time: draft.start_time,
                                },
                            })
                            .unwrap_or_else(|t| {
                                warn!(
                                    "Could not resend draft to newly registered user {:?}: {:?}",
                                    &uid, t
                                );
                            });
                            tx.unbounded_send(SPacket {
                                sender: sender.clone(),
                                destination: dest.clone(),
                                time,
                                packet: Packet::Edit {
                                    uuid: draft.id,
                                    content: draft.content.clone(),
                                    editing_draft: true,
                                },
                            })
                            .unwrap_or_else(|t| {
                                warn!(
                                    "Could not resend draft to newly registered user {:?}: {:?}",
                                    &uid, t
                                );
                            });
                        }
                    }
                }
            }
        }

        // give the receiver so they can talk to the server
        Ok(rx)
    }

    pub fn deregister(&mut self, uid: &UserId) {
        // make sure they're disconnected so we can't send anything to them
        self.open_senders.remove(uid);

        // remove all their drafted messages (not saving them)
        // and notify the clients they were sending them to
        let mut drafts_to_remove = vec![];
        let current_time: Timestamp = get_current_time();
        for ((sender, dest), draft) in self.current_drafts.iter() {
            if sender == uid {
                drafts_to_remove.push((sender.clone(), dest.clone()));
                match dest {
                    Destination::User(to) => {
                        if let Some(tx) = self.open_senders.get(to) {
                            tx.unbounded_send(SPacket {
                                sender: uid.clone(),
                                destination: dest.clone(),
                                time: current_time,
                                packet: Packet::DiscardDraft { uuid: draft.id },
                            })
                            .unwrap_or_else(|err| {
                                warn!(
                                    "Unable to send DiscardDraft packet to receiver {:?}: {:?}",
                                    to, err
                                )
                            });
                        }
                    }
                }
            }
        }

        for k in drafts_to_remove {
            self.current_drafts.remove(&k);
        }
    }

    fn flush_backlog(&mut self, user_id: &UserId) -> Result<(), ServerError> {
        if let Some(tx) = self.open_senders.get(user_id) {
            if let Some(mut backlog) = self.backlog.remove(user_id) {
                while let Some(msg) = backlog.pop_front() {
                    match tx.unbounded_send(msg) {
                        Ok(_) => {}
                        Err(_e) => return Err(ServerError::TrySendError(user_id.clone())),
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn process_message(&mut self, msg: SPacket) -> Result<bool, ServerError> {
        self.process_message_internal(msg)
    }

    /// Create necessary extra packets to pass messages along to everyone that needs it.
    /// Also maintain state with storage.
    fn process_message_internal(&mut self, msg: SPacket) -> Result<bool, ServerError> {
        // route and re-send it
        let (to, _from) = msg.get_to_from();
        let to = match to {
            Destination::User(uid) => uid,
        };
        self.flush_backlog(&to)?; // will only go if sender exists
        let mut disconnected = false;
        let mut try_send = |msg| {
            if let Some(tx) = self.open_senders.get(&to) {
                // then, send the message
                match tx.unbounded_send(msg) {
                    Ok(_) => return Ok(None),
                    Err(e) => {
                        if e.is_disconnected() {
                            disconnected = true;
                            Ok(Some(e.into_inner())) // retrieve the message
                        } else {
                            return Err(ServerError::TrySendError(to.clone()));
                        }
                    }
                }
            } else {
                Ok(Some(msg))
            }
        };

        let SPacket {
            sender,
            destination,
            packet,
            time,
        } = msg;
        let current_time = get_current_time();
        let draft_key = (sender.clone(), destination.clone());

        let backlog_packets = match packet {
            Packet::StartDraft => {
                let uuid = make_uuid();
                self.current_drafts.insert(
                    draft_key.clone(),
                    Draft {
                        content: String::new(),
                        id: uuid,
                        start_time: current_time,
                    },
                );
                let p1 = SPacket {
                    sender: sender.clone(),
                    destination: destination.clone(),
                    time: current_time,
                    packet: Packet::NewDraft {
                        uuid,
                        start_time: current_time,
                    },
                };

                // inform sender of their draft's info
                let p2 = SPacket {
                    sender: sender.clone(),
                    destination: Destination::User(sender.clone()), // inform sender!
                    time: current_time,
                    packet: Packet::NewDraft {
                        uuid,
                        start_time: current_time,
                    },
                };
                for p in vec![p1, p2] {
                    try_send(p)?;
                }
                vec![]
            }
            Packet::EndDraft { content, uuid } => {
                if let Some(draft) = self.current_drafts.remove(&draft_key) {
                    self.storage
                        .add_message(
                            draft.into_message(sender.clone(), current_time),
                            destination.clone(),
                        )
                        .unwrap_or_else(|e| {
                            warn!("Unable to end draft on message {}: {:?}", uuid, e)
                        });
                }
                let p1 = SPacket {
                    sender: sender.clone(),
                    destination: destination.clone(),
                    time: current_time,
                    packet: Packet::EndDraft {
                        content: content.clone(),
                        uuid,
                    },
                };
                let p2 = SPacket {
                    sender: sender.clone(),
                    destination: Destination::User(sender.clone()),
                    time: current_time,
                    packet: Packet::EndDraft { content, uuid },
                };
                let mut unsent = vec![];
                for p in vec![p1, p2] {
                    if let Some(p) = try_send(p)? {
                        unsent.push(p);
                    }
                }
                unsent
            }
            // all of these just echo
            // Packet::NewMessage { .. } => {}
            // Packet::DraftInfo { .. } => {}
            Packet::Edit {
                content,
                uuid,
                editing_draft,
            } => {
                if let Some(draft) = self.current_drafts.get_mut(&draft_key) {
                    if draft.id == uuid {
                        draft.content = content.clone();
                    }
                } else if !editing_draft {
                    let room_id = draft_key.into();
                    if let Ok(room) = self.storage.get_room_mut(&room_id) {
                        room.edit_message(uuid, content.clone())
                            .unwrap_or_else(|err| {
                                warn!("Unable to edit message: {:?}", err);
                            });
                    }
                }
                let p1 = SPacket {
                    sender,
                    destination,
                    time,
                    packet: Packet::Edit {
                        content,
                        uuid,
                        editing_draft,
                    },
                };
                try_send(p1)?;
                vec![]
            }
            packet => try_send(SPacket {
                sender,
                destination,
                time,
                packet,
            })?
            .map(|_p| vec![])
            .unwrap_or(vec![]),
        };
        for bp in backlog_packets {
            warn!("Message added to backlog: {:?}", &bp);
            self.backlog
                .entry(to.clone())
                .or_default()
                .push_back(bp);
        }
        if disconnected {
            // if they disconnect, remove the sending channel
            self.deregister(&to);
        }
        Ok(false)
    }
}

impl From<MessageDAOError> for ServerError {
    fn from(value: MessageDAOError) -> Self {
        ServerError::DAOError(value)
    }
}

pub struct ShutdownHandler(Mutex<Vec<JoinHandle<()>>>);
impl ShutdownHandler {
    pub fn new(handle: JoinHandle<()>) -> ShutdownHandler {
        ShutdownHandler(Mutex::new(vec![handle]))
    }

    pub fn join(&self) {
        let mut handles = self.0.lock().unwrap();
        for thread_handle in handles.drain(..) {
            match thread_handle.join() {
                Ok(()) => {}
                Err(err) => {
                    panic::resume_unwind(err); // recommended??
                    // eprintln!("Message server did not unwind successfully!");
                }
            }
        }
    }
}

#[rocket::async_trait]
impl Fairing for ShutdownHandler {
    fn info(&self) -> Info {
        Info {
            name: "Message Server Shutdown Handler",
            kind: rocket::fairing::Kind::Shutdown,
        }
    }

    async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
        self.join();
    }
}

#[cfg(test)]
mod test {
    use crate::identity::make_user_id;
    use crate::message_server;
    use crate::packet::{Destination, Packet, SPacket};
    use crate::storage::memory_storage::MemoryMessageDatabase;
    use rocket::futures::StreamExt;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use uuid::Uuid;

    // stolen from https://github.com/rust-lang/book/blob/main/packages/trpl/src/lib.rs
    pub fn run<F: Future>(future: F) -> F::Output {
        let rt = rocket::tokio::runtime::Runtime::new().unwrap();
        rt.block_on(future)
    }

    #[test]
    fn test_server_start() {
        // usually the message server runs on a separate thread,
        // but for this example it was easier to get it set up on the main
        // thread... hopefully this problem is not in the main server

        let server = message_server::MessageServer::new(MemoryMessageDatabase::new());
        let server = Arc::new(Mutex::new(server));
        let (s_sender, s_receiver) = std::sync::mpsc::channel();
        let uid_a = make_user_id("A".to_string());
        let uid_b = make_user_id("B".to_string());

        let inner_server = Arc::clone(&server);
        let inner_uid_a = uid_a.clone();
        let inner_uid_b = uid_b.clone();
        let handle = thread::spawn(move || {
            s_sender
                .send(SPacket {
                    sender: inner_uid_a.clone(),
                    destination: Destination::User(inner_uid_b.clone()),
                    time: 0,
                    packet: Packet::NewMessage {
                        uuid: Uuid::new_v4(),
                        content: "howdy".to_string(),
                        start_time: 0,
                        end_time: 0,
                    },
                })
                .unwrap();

            println!("packet sent");
            let mut rx_b = inner_server
                .lock()
                .unwrap()
                .register(inner_uid_b.clone())
                .unwrap();
            run(async move {
                let packet = rx_b.next().await.unwrap();
                match &packet.packet {
                    Packet::NewMessage { content: msg, .. } => {
                        println!("message from {:?}: \"{}\"", packet.sender, msg);
                        msg.clone()
                    }
                    _ => panic!("crashout"),
                }
            })
        });

        for spacket in s_receiver {
            println!("Acquiring server lock...");
            let mut s = server.lock().unwrap();
            println!("Got lock.");
            match s.process_message(spacket) {
                Ok(sent) => {
                    if sent {
                        println!("Message routed & sent!")
                    } else {
                        println!("Message added to backlog.")
                    }
                }
                Err(e) => {
                    eprintln!("Error occurred while sending message: {e:?}")
                }
            };
            println!("Releasing lock...");
        }

        let result = handle.join().unwrap();
        assert_eq!(result, "howdy".to_string());
    }

    #[test]
    fn channel() {
        let (tx, rx) = rocket::futures::channel::mpsc::unbounded();
        drop(rx);
        tx.unbounded_send(3).unwrap();
    }
}
