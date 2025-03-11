import React, { useEffect, useRef, useState } from 'react';

/*
// ------------------------- Web Packet Rust Spec -----------------------------
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
    EndDraft {
        #[serde(with = "uuid::serde::compact")]
        uuid: Uuid
    },
    Edit {
        #[serde(with = "uuid::serde::compact")]
        uuid: Uuid,
        content: String,
    },
}
*/

type Uuid = string;
type Timestamp = number;

interface WebPacket {
  content: Packet,
  destination: WebDest,
  sender?: string,
  timestamp?: Timestamp,
}

interface WebDest {
  User: string
  // Group: Uuid // sometime later for group chats
}

interface Packet {
  // all of the variants are optional, but at least one should be present
  NewMessage?: {
    content: string
  },
  // SyncMessage: Uuid, String // to be used to sync database w/ chats
  StartDraft?: {},
  EndDraft?: {
    uuid: Uuid
  },
  Edit?: {
    uuid: Uuid,
    content: string
  }
}


// -------------------- frontend use --------------------
interface Message {
  sender: string,
  destination: WebDest,
  message: string,
}

const Login: React.FC = () => {
  const [username, setUsername] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [triedLogin, setTriedLogin] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [messages, setMessages] = useState<Message[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  const processPacket = (wpacket: WebPacket) => {
    const packet = wpacket.content;
    if (packet.NewMessage) {
      setMessages(prevMessages => [...prevMessages, {
        sender: wpacket.sender || '<unknown>',
        destination: wpacket.destination,
        message: packet.NewMessage?.content || ''
      }]);
    }
  }

  const handleLogin = () => {
    if (username) {
      setSubmitted(true);
    }
  };

  useEffect(() => {
    if (submitted && !isConnected && (!wsRef.current || wsRef.current.readyState === WebSocket.CLOSED)) {
      const WS_URL = `ws://localhost:8000/updates/${username}`;
      console.log('connection string: ' + WS_URL);
      const ws = new WebSocket(WS_URL);
      console.log(ws)
      wsRef.current = ws;
      ws.onmessage = (event: MessageEvent) => {
        let webpacket = JSON.parse(event.data);
        processPacket(webpacket);
      }
      ws.onopen = () => {
        setIsConnected(true);
        console.log('✅ Connected to server');
      }
      ws.onclose = () => {
        setIsConnected(false);
        console.log('❌ Disconnected from server');
        setSubmitted(false);
      }
      ws.onerror = (error) => {
        console.error('⚠️ Error:', error);
      }
      setTriedLogin(true);
    }

    return () => {
      if (wsRef.current) {
        wsRef.current?.close();
      }
    };
  }, [submitted, username]);

  

  const showMessage = (message: Message) => {
    return (
      <div>
        <div>from: {message.sender}</div>
        <div>{message.message}</div>
      </div>
    )
  }

  const sendMessage = (content: string, to: string) => {
    const message: WebPacket = {
      destination: { User: to },
      content: {
        NewMessage: { content }
      }
    }
    if (wsRef.current) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.error('Unable to send message, websocket is not connected');
    }
  }

  return (
    <div>
      {isConnected ? (<div>
        <p> Connected! </p>
        Message room for {username}
        <br />
        <div>
          {messages.map((message, index) => (
            <div key={index}>
              {showMessage(message)}
            </div>
          ))}
          Send to: <input type="text" id='send_to' />
          Message: <input type="text" id='message_content' />
          <button onClick={
            () => {
              const send_to = (document.getElementById('send_to') as HTMLInputElement).value;
              const message_content = (document.getElementById('message_content') as HTMLInputElement).value;
              sendMessage(message_content, send_to);
              // clear input
              (document.getElementById('message_content') as HTMLInputElement).value = '';
            }
          }>Send</button>
          <button onClick={
            () => {
              wsRef.current?.close();
            }
          }>Disconnect</button>
        </div>
      </div>
      ) : (
        <div>
          <h2>Login</h2>
          {triedLogin ? <p>⚠️ Failed to connect. Retry </p> : null}
          <input
            type="text"
            placeholder="Enter your username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
          <button onClick={handleLogin}>Submit</button>
        </div>
      )}
    </div>
  );
};

export default Login;