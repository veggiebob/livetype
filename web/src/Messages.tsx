import React from 'react';
import { Message } from './protocol';

type MessagesProps = {
  messages: Message[];
};

const Messages: React.FC<MessagesProps> = ({ messages }) => {
  return (
    <div>
      {messages.map((message) => (
        <div key={message.uuid}>
          <p><strong>{message.sender}:</strong> {message.content}</p>
        </div>
      ))}
    </div>
  );
};

export default Messages;