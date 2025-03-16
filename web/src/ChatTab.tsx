import React from 'react';
import { UserId, Message, Draft } from './protocol';
import './ChatTab.css';
import DraftMessage from './DraftMessage';

interface ChatTabProps {
  sender: UserId,
  messages: Message[],
  username: string,
  draft: Draft | undefined,
}

interface TimeEvent {
  time: number,
  sender: UserId,
  m_index: number,
  start: boolean,
}

interface DisplayMessage {
  start_row: number,
  end_row: number,
  message: Message,
}

interface Spacer {
  start_row: number,
  end_row: number,
  sender: UserId,
  key: string,
}

interface RenderMessage {
  Spacer?: Spacer,
  DisplayMessage?: DisplayMessage
}

function getStartRow(rm: RenderMessage): number {
  if (rm.DisplayMessage) {
    return rm.DisplayMessage.start_row;
  } else if (rm.Spacer) {
    return rm.Spacer.start_row;
  } else {
    return 0;
  }
}

const getTimeString = (time: number) => new Date(time / 1000).toTimeString();

function mergeThreads(threads: RenderMessage[][]): RenderMessage[] {
  let bigThread: RenderMessage[] = threads.flat();
  bigThread.sort((a, b) => getStartRow(a) - getStartRow(b));
  return bigThread;
}

const ChatTab: React.FC<ChatTabProps> = ({ sender, messages, username, draft }) => {
  // create a list of time stamps that correspond to the index of the message, and whether it's a start or end time
  let time_events: TimeEvent[] = [];
  messages.map((message, index) => {
    time_events.push({time: message.start_time, m_index: index, start: true, sender: message.sender});
    time_events.push({time: message.end_time, m_index: index, start: false, sender: message.sender});
  });
  // sort the time events by time
  time_events.sort((a, b) => a.time - b.time);
  let display_messages: Map<number, DisplayMessage> = new Map();
  for (let i = 0; i < time_events.length; i++) {
    let event = time_events[i];
    let cell = i + 1;
    let dm = display_messages.get(event.m_index);
    if (dm) {
      if (event.start) {
        dm.start_row = cell;
      } else {
        dm.end_row = cell + 1;
      }
    } else {
      if (event.start) {
        display_messages.set(event.m_index, {
          message: messages[event.m_index],
          start_row: cell,
          end_row: cell + 1
        });
      }
    }
  }

  let spacer_key = 0;
  function getPersonalThread(dms: Map<number, DisplayMessage>, user: UserId): RenderMessage[] {
    let me_thread: RenderMessage[] = [];
    let cell: number | null = null;
    Array.from(dms.entries())
      .filter(([_n, dm]) => dm.message.sender === user)
      .sort(([_a, a], [_b, b]) => a.start_row - b.start_row)
      .forEach(([_n, dm]) => {
        console.log("cell is", cell);
        // if (cell !== null && dm.start_row > cell + 1) {
        //   me_thread.push({
        //     Spacer: {
        //       start_row: cell,
        //       end_row: dm.start_row,
        //       sender: user,
        //       key: `spacer-${spacer_key++}`
        //     }
        //   });
        // }
        me_thread.push({
          DisplayMessage: dm
        })
        cell = dm.end_row;
      });
    return me_thread;
  }

  const me_thread = getPersonalThread(display_messages, username);
  const you_thread = getPersonalThread(display_messages, sender);
  const both_threads = mergeThreads([me_thread, you_thread]);


  const getBubbleColor = (sender: string) => {
    if (sender === username) {
      return "var(--color-2)";
    } else {
      return "var(--color-1)";
    }
  }
  console.log("me", me_thread);
  console.log("you", you_thread);
  console.log(both_threads);
  return (
    <div>
      <h2>Messages from {sender}</h2>
      <div className="messages-container">
        {both_threads
          .map((rm) => {
          if (rm.Spacer) {
            let m_class = "spacer";
            if (rm.Spacer.sender === username) {
              m_class += " m-user";
            } else {
              m_class += " m-friend";
            }
            return (
              <div 
                key={rm.Spacer.key}
                className={m_class}
                style={{
                  gridRowStart: rm.Spacer.start_row,
                  gridRowEnd: rm.Spacer.end_row,
                }}
              >
                spacer
              </div>
            );
          }
          const dm = rm.DisplayMessage!;
          const message = dm.message;
          let from_me = message.sender === username;
          let message_class = "message";
          if (from_me) {
            message_class += " m-user";
          } else {
            message_class += " m-friend";
          }
          let e = (
            <div 
              key={message.uuid}
              className={message_class}
              style={{
                backgroundColor: getBubbleColor(message.sender),
                gridRowStart: dm.start_row,
                gridRowEnd: dm.end_row,
              }}
            >
              {message.content}
            </div>
          );
          return e;
        })}
      </div>
      {draft && <DraftMessage draft={draft} sender={sender} />}
    </div>
  )
};

export default ChatTab;
