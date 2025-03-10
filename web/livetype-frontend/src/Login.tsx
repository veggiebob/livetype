import React, { useEffect, useRef, useState } from 'react';

interface UPacket {
  sender: string,
  message: string,
  destination: string,
}

const Login: React.FC = () => {
  const [username, setUsername] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [triedLogin, setTriedLogin] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [messages, setMessages] = useState<UPacket[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

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
        let message = JSON.parse(event.data);
        setMessages([...messages, message]);
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

  

  const showMessage = (message: UPacket) => {
    return (
      <div>
        <div>from: {message.sender}</div>
        <div>{message.message}</div>
      </div>
    )
  }

  const sendMessage = (content: string, to: string) => {
    const message: UPacket = {
      sender: username,
      message: content,
      destination: to,
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