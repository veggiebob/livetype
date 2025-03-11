use crate::identity::UserId;
use rocket::fairing::{Fairing, Info};
use rocket::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rocket::{Orbit, Rocket};
use std::collections::{HashMap, VecDeque};
use std::panic;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use crate::packet::{Destination, RoutingInfo};

pub struct MessageServer<M> {
    open_senders: HashMap<UserId, UnboundedSender<M>>,
    backlog: HashMap<UserId, VecDeque<M>>,
}

#[derive(Debug)]
pub enum ServerError {
    AlreadyInUse(UserId),
    TrySendError(UserId),
}

impl<M: RoutingInfo + Send + 'static> MessageServer<M> {
    pub fn new() -> Self {
        MessageServer {
            backlog: HashMap::new(),
            open_senders: HashMap::new(),
        }
    }
    pub fn start(/* config? */) -> (Sender<M>, Arc<Mutex<Self>>, ShutdownHandler) {
        let server = Self::new();
        let server = Arc::new(Mutex::new(server));
        let server2 = Arc::clone(&server);
        let (tx, rx) = mpsc::channel();
        println!("Server started!");
        let handle = std::thread::spawn(move || {
            for spacket in rx {
                let mut s = server.lock().unwrap();
                match s.process_message(spacket) {
                    Ok(sent) => {
                        if sent {
                            info!("Message routed & sent!")
                        } else {
                            info!("Message added to backlog.")
                        }
                    }
                    Err(e) => {
                        error!("Error occurred while sending message: {e:?}")
                    }
                };
            }
        });
        (tx, server2, ShutdownHandler::new(handle))
    }
    pub fn register(&mut self, uid: UserId) -> Result<UnboundedReceiver<M>, ServerError> {
        if self.open_senders.contains_key(&uid) {
            return Err(ServerError::AlreadyInUse(uid));
        }
        let (tx, rx) = rocket::futures::channel::mpsc::unbounded();
        self.open_senders.insert(uid.clone(), tx);
        self.flush_backlog(&uid)?; // send them the messages they missed
        Ok(rx)
    }

    pub fn deregister(&mut self, uid: &UserId) {
        self.open_senders.remove(uid);
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

    pub fn process_message(&mut self, msg: M) -> Result<bool, ServerError> {
        // route and re-send it
        let (to, _from) = msg.get_to_from();
        let to = match to {
            Destination::User(uid) => uid
        };
        self.flush_backlog(&to)?; // will only go if sender exists
        let mut disconnected = false;
        let msg = if let Some(tx) = self.open_senders.get(&to) {
            // then, send the message
            match tx.unbounded_send(msg) {
                Ok(_) => return Ok(true),
                Err(e) => {
                    if e.is_disconnected() {
                        disconnected = true;
                        e.into_inner() // retrieve the message
                    } else {
                        return Err(ServerError::TrySendError(to));
                    }
                }
            }
        } else {
            msg
        };
        self.backlog.entry(to.clone()).or_default().push_back(msg);
        if disconnected {
            // if they disconnect, remove the sending channel
            self.open_senders.remove(&to);
        }
        Ok(false)
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
    use rocket::futures::{StreamExt};
    use std::sync::{Arc, Mutex};
    use std::thread;

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

        let server: message_server::MessageServer<SPacket> = message_server::MessageServer::new();
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
                    packet: Packet::NewMessage { content: "howdy".to_string() },
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
                    Packet::NewMessage { content: msg } => {
                        println!("message from {:?}: \"{}\"", packet.sender, msg);
                        msg.clone()
                    }
                    _ => panic!("crashout")
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
        assert_eq!(
            result,
            "howdy".to_string()
        );
    }

    #[test]
    fn channel() {
        let (tx, rx) = rocket::futures::channel::mpsc::unbounded();
        drop(rx);
        tx.unbounded_send(3).unwrap();
    }
}
