import React from 'react';
import { Draft, Message, UserId } from './protocol';
import ChatTab from './ChatTab';

type MessagesProps = {
  messages: Message[],
  drafts: Map<UserId, Draft>,
  username: string,
};

const Messages: React.FC<MessagesProps> = ({ messages, drafts, username }) => {
  let users = new Set<string>();
  messages.forEach((message) => {
    users.add(message.sender);
    if (message.destination.User) {
      users.add(message.destination.User);
    }
  });
  drafts.forEach((draft, sender) => {
    users.add(sender);
  });
  users.delete(username); // don't have a chat with yourself
  return (
    <div>
      {Array.from(users).map((sender) => {
        let draft = drafts.get(sender);
        let s_messages = messages.filter((message) => {
          return message.destination.User === sender || message.sender === sender;
        });
        return (
          <ChatTab 
            key={sender} 
            username={username} 
            sender={sender} 
            messages={s_messages}
            draft={draft}
          />)
      })}
    </div>
  )
};

export default Messages;