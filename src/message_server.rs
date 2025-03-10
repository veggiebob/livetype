use crate::identity::UserId;
use rocket::fairing::{Fairing, Info};
use rocket::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rocket::{Orbit, Rocket};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::panic;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

pub struct MessageServer<M> {
    open_senders: HashMap<UserId, UnboundedSender<M>>,
    backlog: HashMap<UserId, VecDeque<M>>,
}

pub trait RoutingInfo<U> {
    fn get_to_from(&self) -> (U, U);
}

#[derive(Debug)]
pub enum ServerError {
    AlreadyInUse(UserId),
    TrySendError(UserId),
}

impl<M: RoutingInfo<UserId> + Send + 'static> MessageServer<M> {
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
                            println!("Message routed & sent!")
                        } else {
                            println!("Message added to backlog.")
                        }
                    }
                    Err(e) => {
                        eprintln!("Error occurred while sending message: {e:?}")
                    }
                };
            }
            panic!("Finished server thread!")
        });
        (tx, server2, ShutdownHandler::new(handle))
    }
    pub fn new() -> Self {
        MessageServer {
            backlog: HashMap::new(),
            open_senders: HashMap::new(),
        }
    }
    pub fn register(&mut self, uid: UserId) -> Result<UnboundedReceiver<M>, ServerError> {
        if self.open_senders.contains_key(&uid) {
            return Err(ServerError::AlreadyInUse(uid));
        }
        let (tx, rx) = rocket::futures::channel::mpsc::unbounded();
        self.open_senders.insert(uid, tx);
        Ok(rx)
    }
    pub fn process_message(&mut self, msg: M) -> Result<bool, ServerError> {
        // route and re-send it
        let (to, _from) = msg.get_to_from();
        match self.open_senders.entry(to.clone()) {
            Entry::Occupied(entry) => {
                let to2 = to.clone();
                // first, empty the backlog
                if let Some(mut backlog) = self.backlog.remove(&to) {
                    while let Some(msg) = backlog.pop_front() {
                        match entry.get().unbounded_send(msg) {
                            Ok(_) => {}
                            Err(_e) => return Err(ServerError::TrySendError(to2)),
                        }
                    }
                }
                // then, send the message
                match entry.get().unbounded_send(msg) {
                    Ok(_) => Ok(true),
                    Err(_e) => Err(ServerError::TrySendError(to2)),
                }
            }
            Entry::Vacant(_v) => {
                // we can't create a connection if one doesn't already exist
                // so, put it in the backlog
                self.backlog.entry(to.clone()).or_default().push_back(msg);
                Ok(false)
            }
        }
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
    use crate::packet::SPacket;
    use rocket::futures::StreamExt;
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
                    message: "howdy".to_string(),
                    destination: inner_uid_b.clone(),
                })
                .unwrap();

            println!("packet sent");
            let mut rx_b = inner_server
                .lock()
                .unwrap()
                .register(inner_uid_b.clone())
                .unwrap();
            run(async move {
                let msg = rx_b.next().await.unwrap();
                println!("message from {:?}: \"{}\"", &msg.sender, &msg.message);
                msg
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
            SPacket {
                sender: uid_a.clone(),
                destination: uid_b.clone(),
                message: "howdy".to_string()
            }
        );
    }
}
