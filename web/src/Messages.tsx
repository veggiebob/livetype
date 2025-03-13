import React from 'react';
import { Draft, Message, UserId } from './protocol';
import ChatTab from './ChatTab';

type MessagesProps = {
  messages: Message[],
  drafts: Map<UserId, Draft>,
  username: string,
};

const Messages: React.FC<MessagesProps> = ({ messages, drafts, username }) => {
  let senders = new Set<string>();
  messages.forEach((message) => {
    senders.add(message.sender);
  });
  drafts.forEach((draft, sender) => {
    senders.add(sender);
  });
  senders.delete(username);
  // return (
  //   <div>
  //     {messages.map((message) => (
  //       <div key={message.uuid}>
  //         <p><strong>{message.sender}:</strong> {message.content}</p>
  //       </div>
  //     ))}
  //   </div>
  // );
  return (
    <div>
      {Array.from(senders).map((sender) => {
        let draft = drafts.get(sender);
        let s_messages = messages.filter((message) => {
          console.log(message.destination.User, sender);
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