#[macro_use]
extern crate rocket;

use crate::packet::{make_server_packet, make_upacket, SPacket};
use identity::{make_user_id};
use packet::UPacket;
use rocket::futures::channel::mpsc::UnboundedReceiver;
use rocket::futures::{SinkExt, StreamExt};
use rocket::tokio::select;
use rocket::{tokio, State};
use rocket_ws::stream::DuplexStream;
use rocket_ws::{Channel, WebSocket};
use std::sync::mpsc::{Sender};
use std::sync::{mpsc, Arc, Mutex};

pub mod message_server;
pub mod packet;
mod identity;

type MessageServer = State<Arc<Mutex<message_server::MessageServer<SPacket>>>>;

#[derive(Clone)]
struct ServerSender(mpsc::Sender<SPacket>);

#[get("/")]
fn index() -> &'static str {
    "Hi!"
}

#[get("/updates/<uid>")]
fn updates<'r>(server: &MessageServer, server_sender: &State<ServerSender>, ws: WebSocket, uid: String) -> Channel<'r> {
    let mut server = server.lock().unwrap();
    let rx = server.register(make_user_id(uid.clone())).unwrap();
    let tx = server_sender.0.clone();
    ws.channel(move |stream| Box::pin(handle_socket(tx, rx, stream, uid)))
}

async fn handle_socket(
    tx: Sender<SPacket>,
    mut rx: UnboundedReceiver<SPacket>,
    channel: DuplexStream,
    uid: String,
) -> rocket_ws::result::Result<()> {
    let (mut sender, mut receiver) = channel.split();

    // Receiving task (handles incoming messages from the WebSocket)
    let r_uid = uid.clone();
    let receive_task = tokio::spawn(async move {
        let r_uid = make_user_id(r_uid);
        while let Some(Ok(msg)) = receiver.next().await {
            // convert to SPacket
            match UPacket::try_from(msg) {
                Ok(upacket) => tx.send(make_server_packet(upacket, r_uid.clone())).unwrap(),
                Err(e) => panic!("Missed packet because {e:?}"),
            }
        }
    });

    // Sending task (handles outgoing messages)
    let send_task = tokio::spawn(async move {
        while let Some(server_message) = rx.next().await {
            // convert to UPacket
            let upacket = make_upacket(server_message);
            sender.send(upacket.try_into().unwrap()).await.unwrap();
        }
    });

    // Wait for either task to complete
    select! {
        _ = receive_task => println!("Receiver closed for {}", uid.clone()),
        _ = send_task => println!("Sender closed for {}", uid),
    }

    Ok(())
}

#[launch]
fn rocket() -> _ {
    let (s_sender, server, shutdown_server) = message_server::MessageServer::start();
    rocket::build()
        .attach(shutdown_server)
        .manage(ServerSender(s_sender))
        .manage(server)
        .mount("/", routes![index, updates])
}
