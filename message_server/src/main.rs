#[macro_use]
extern crate rocket;

use crate::packet::{make_server_packet, make_webpacket, SPacket};
use identity::make_user_id;
use log::{error, info, warn};
use packet::WebPacket;
use rocket::futures::channel::mpsc::UnboundedReceiver;
use rocket::futures::{SinkExt, StreamExt};
use rocket::response::status;
use rocket::tokio::select;
use rocket::{tokio, State};
use rocket_ws::stream::DuplexStream;
use rocket_ws::{Channel, Message, WebSocket};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use crate::storage::memory_storage::MemoryMessageDatabase;

mod identity;
pub mod message_server;
pub mod packet;
mod storage;
mod protocol;

type MessageServer = State<Arc<Mutex<message_server::MessageServer<MemoryMessageDatabase>>>>;

#[derive(Clone)]
struct ServerSender(mpsc::Sender<SPacket>);

#[get("/")]
fn index() -> &'static str {
    "Hi!"
}

#[get("/updates/<uid>")]
fn updates<'r>(
    server: &'r MessageServer,
    server_sender: &State<ServerSender>,
    ws: WebSocket,
    uid: &'r str,
) -> Result<Channel<'r>, status::Forbidden<&'static str>> {
    let server2 = server;
    let mut server = server.lock().unwrap();
    let rx = server
        .register(make_user_id(uid.to_string()))
        .map_err(|_| status::Forbidden("Already registered"))?;
    let tx = server_sender.0.clone();
    Ok(ws.channel(move |stream| Box::pin(handle_socket(server2, tx, rx, stream, uid.to_string()))))
}

async fn handle_socket(
    server: &MessageServer,
    tx: Sender<SPacket>,
    mut rx: UnboundedReceiver<SPacket>,
    channel: DuplexStream,
    uid: String,
) -> rocket_ws::result::Result<()> {
    let (mut sender, mut receiver) = channel.split();
    let user_id = make_user_id(uid.clone());
    info!("Registered {:?}", &user_id);
    // Receiving task (handles incoming messages from the WebSocket)
    let r_uid = uid.clone();
    let receive_task = tokio::spawn(async move {
        let r_uid = make_user_id(r_uid);
        while let Some(Ok(msg)) = receiver.next().await {
            // convert to SPacket
            match msg {
                Message::Close(_c) => {
                    info!("Closing connection.");
                    break;
                }
                msg => match WebPacket::try_from(msg) {
                    Ok(upacket) => tx.send(make_server_packet(upacket, r_uid.clone())).unwrap(),
                    Err(e) => {
                        error!("Unable to parse upacket: {:?}", e);
                    }
                }
            }
        }
    });

    // Sending task (handles outgoing messages)
    let send_task = tokio::spawn(async move {
        while let Some(server_message) = rx.next().await {
            // convert to UPacket
            let upacket = make_webpacket(server_message);
            sender.send(upacket.try_into().unwrap()).await.unwrap();
        }
    });

    // Wait for either task to complete
    select! {
        _ = receive_task => info!("Channel closed from receiver end for {}", uid.clone()),
        _ = send_task => info!("Channel closed from sender end for {}", uid),
    }

    match server.lock() {
        Ok(mut s) => {
            s.deregister(&user_id);
            info!("Deregistered {:?}", &user_id);
        }
        Err(_e) => error!("Unable to unlock server to deregister {:?}", &user_id),
    };

    Ok(())
}

#[launch]
fn rocket() -> _ {
    let (s_sender, server, shutdown_server) = message_server::MessageServer::start(MemoryMessageDatabase::new());
    rocket::build()
        .attach(shutdown_server)
        .manage(ServerSender(s_sender))
        .manage(server)
        .mount("/", routes![index, updates])
}
