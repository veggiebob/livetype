import React from 'react';
import './GridTest.css';
import Messages from './Messages';
import { Draft, Message, UserId } from './protocol';

/*
interface Message {
  sender: UserId,
  destination: WebDest,
  content: string,
  uuid: Base64Uuid,
  start_time: Timestamp,
  end_time: Timestamp,
}
  */
let id_counter = 0;
function new_id(): string {
  return (id_counter++).toString();
}
const GridTest: React.FC = () => {
  const me: string = "Alice";
  const you: string = "Bob";
  const messages: Message[] = [
    {
      sender: you,
      destination: { User: me },
      content: "Hi, Alice!",
      uuid: new_id(),
      start_time: 2,
      end_time: 6
    },
    {
      sender: me,
      destination: { User: you },
      content: "Hello, Bob!",
      uuid: new_id(),
      start_time: 5,
      end_time: 7
    },
    {
      sender: you,
      destination: {User: me},
      content: "I was wondering if you would like to get some tea later?",
      uuid: new_id(),
      start_time: 8,
      end_time: 10
    }
  ];
  const drafts: Map<UserId, Draft> = new Map();
  drafts.set(you, { content: "I wanted to say that..." });
  return (
    <Messages messages={messages} drafts={drafts} username={me} />
  );
  // return (
  //   <div className="message-grid">
  //     <div className="">abc</div>
  //     <div className="">def</div>
  //   </div>
  // )
};

export default GridTest;