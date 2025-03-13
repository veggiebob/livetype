import React from 'react';
import { UserId, Message, Draft } from './protocol';
import DraftMessage from './DraftMessage';

interface ChatTabProps {
  sender: UserId,
  messages: Message[],
  username: string,
  draft: Draft | undefined,
}

const ChatTab: React.FC<ChatTabProps> = ({ sender, messages, username, draft }) => {
  return (
    <div>
      <h2>Messages from {sender}</h2>
      <ul>
        {messages.map(message => {
          let sent = message.sender === username;
          return (
          <li key={message.uuid} className={sent ? "sent-message" : "received-message"}>
            {sent ? (<strong>you:</strong>) : (<strong>{message.sender}:</strong>)} <span>{message.content}</span>
            <p>
              <small>{new Date(message.start_time / 1000).toTimeString()}</small>
            </p>
          </li>
        );})}
      </ul>
      {draft && (
        <DraftMessage draft={draft} sender={username} />
      )}
    </div>
  );
};

export default ChatTab;
