import React, { useEffect, useReducer, useRef, useState } from 'react';
import { WebPacket, Message, Draft, UserId, assertUserId, assertUuid, uuid2str, str2uuid, Base64Uuid, WebDest } from './protocol';
import Messages from './Messages';
import DraftMessage from './DraftMessage';


const present = <T,>(value: T | null | undefined): T => {
  if (value === null || value === undefined) {
    throw new Error('value is null or undefined');
  }
  return value;
};

interface State {
  username: string,
  messages: Message[],
  currentDraft: Draft,
  workingDraft: boolean,
  workingDest?: WebDest,
  senderDrafts: Map<UserId, Draft>,
}

interface Action {
  type: string,
  payload: any,
}

const initialState: State = {
  username: '',
  messages: [],
  currentDraft: {
    uuid: undefined,
    content: '',
    start_time: undefined,
    end_time: undefined
  },
  workingDraft: false,
  senderDrafts: new Map<UserId, Draft>(),
}

const ACTIONS = {
  UPDATE_USERNAME: 'UPDATE_USERNAME',
  ADD_MESSAGE: 'ADD_MESSAGE',
  UPDATE_CURRENT_DRAFT: 'UPDATE_CURRENT_DRAFT',
  UPDATE_SENDER_DRAFTS: 'UPDATE_SENDER_DRAFTS',
  UPDATE_MESSAGE: 'UPDATE_MESSAGE',
  END_DRAFT: 'END_DRAFT',
  UPDATE_WORKING_DRAFT: 'UPDATE_WORKING_DRAFT',
  SEND_DRAFT: 'SEND_DRAFT',
  RESET: 'RESET',
}

function reducer(state: State, action: Action): State {
  // if action.payload is a function, call it with the current state
  let payload = action.payload;
  if (typeof payload === 'function') {
    payload = payload(state);
  }
  console.log('dispatching action', action, 'with payload ', payload);
  switch (action.type) {
    case ACTIONS.UPDATE_WORKING_DRAFT:
      return {
        ...state,
        workingDraft: payload
      };
    case ACTIONS.UPDATE_USERNAME:
      return {
        ...state,
        username: payload.username
      };
    case ACTIONS.ADD_MESSAGE:
      return {
        ...state,
        messages: [...state.messages, payload]
      };
    case ACTIONS.UPDATE_MESSAGE:
      return {
        ...state,
        messages: state.messages.map(message => {
          if (message.uuid === payload.uuid) {
            return {
              ...message,
              content: payload.content
            }
          } else {
            return message;
          }
        })
      };
    case ACTIONS.UPDATE_CURRENT_DRAFT:
      return {
        ...state,
        currentDraft: payload
      };
    case ACTIONS.UPDATE_SENDER_DRAFTS:
      return {
        ...state,
        senderDrafts: payload
      };
    case ACTIONS.END_DRAFT:
      let packet: WebPacket = payload.packet;
      let uuid: Base64Uuid = payload.uuid;
      // if the uuid matches the current draft, add a message and clear the current draft
      // if it matches a sender draft, add a message and remove the sender draft
      console.log('new message: ', packet);
      if (state.currentDraft.uuid === uuid) {
        return {
          ...state,
          currentDraft: {
            uuid: undefined,
            content: '',
            start_time: undefined,
            end_time: undefined
          },
          messages: [...state.messages, {
            sender: state.username,
            destination: {
              User: assertUserId(state.workingDest?.User)
            },
            uuid: uuid,
            content: present(packet.content.EndDraft?.content),
            start_time: present(state.currentDraft.start_time),
            end_time: present(packet.timestamp),
          }],
          workingDraft: false
        }
      } else {
        let senderDrafts = new Map(state.senderDrafts);
        for (let [sender, draft] of [...senderDrafts.entries()]) {
          if (draft.uuid === uuid) {
            senderDrafts.delete(sender);
            return {
              ...state,
              senderDrafts: senderDrafts,
              messages: [...state.messages, {
                sender: assertUserId(packet.sender),
                destination: packet.destination,
                uuid: uuid,
                content: present(packet.content.EndDraft?.content),
                start_time: present(draft.start_time),
                end_time: present(packet.timestamp),
              }]
            }
          }
        }
        return state;
      }
    case ACTIONS.SEND_DRAFT:
      return state;
    case ACTIONS.RESET:
      return initialState;
    default:
      return state;
  }
}

const Login: React.FC = () => {
  const [username, setUsername] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [triedLogin, setTriedLogin] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [state, dispatch] = useReducer(reducer, initialState);
  const inputRef = useRef<HTMLInputElement>(null);
  const [inputContent, setInputContent] = useState('');
  const sendFieldRef = useRef<HTMLInputElement>(null);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    console.log('current draft', state.currentDraft);
  }, [state]);

  useEffect(() => {
    // update username
    dispatch({ type: ACTIONS.UPDATE_USERNAME, payload: { username } });
  }, [username]);

  const processPacket = (wpacket: WebPacket) => {
    const packet = wpacket.content;
    if (packet.NewMessage) {
      console.log("Received a NewMessage packet", packet.NewMessage);
      const newMessage: Message = {
        sender: assertUserId(wpacket.sender),
        destination: wpacket.destination,
        uuid: uuid2str(present(packet.NewMessage?.uuid)),
        content: present(packet.NewMessage?.content),
        start_time: present(wpacket.timestamp),
        end_time: present(wpacket.timestamp),
      };
      dispatch({ type: ACTIONS.ADD_MESSAGE, payload: newMessage });
    } else if (packet.StartDraft) {
      console.log("Received a StartDraft packet...doing nothing");
    } else if (packet.NewDraft) {
      console.log("Received a NewDraft packet. Uuid: ", packet.NewDraft.uuid);
      // todo: what if you already have a draft??
      if (wpacket.sender === username) {
        console.log('updating current draft uuid');
        dispatch({
          type: ACTIONS.UPDATE_CURRENT_DRAFT,
          payload: (state: State) => ({
            uuid: uuid2str(present(packet.NewDraft?.uuid)),
            content: state.currentDraft?.content,
            start_time: present(wpacket.timestamp),
            end_time: undefined,
          })
        });
      } else {
        console.log('creating new draft for sender', wpacket.sender);
        dispatch({
          type: ACTIONS.UPDATE_SENDER_DRAFTS,
          payload: (state: State) => {
            let senderDrafts = new Map(state.senderDrafts);
            senderDrafts.set(assertUserId(wpacket.sender), {
              uuid: uuid2str(present(packet.NewDraft?.uuid)),
              content: '',
              start_time: present(wpacket.timestamp),
              end_time: undefined,
            });
            return senderDrafts;
          }
        });
      }
    } else if (packet.Edit) {
      // works on any message with the correct uuid
      console.log('receieved edit', packet.Edit);
      let uuid = uuid2str(present(packet.Edit?.uuid));
      let content = present(packet.Edit?.content);
      dispatch({
        type: ACTIONS.UPDATE_MESSAGE,
        payload: { uuid, content }
      });
      dispatch({
        type: ACTIONS.UPDATE_SENDER_DRAFTS,
        payload: (state: State) => {
          let senderDrafts = new Map(state.senderDrafts);
          for (let [sender, draft] of [...senderDrafts.entries()]) {
            if (draft.uuid === uuid) {
              senderDrafts.set(sender, {
                ...draft,
                content
              });
              break;
            }
          }
          return senderDrafts;
        }
      });
    } else if (packet.EndDraft) {
      console.log("received end draft", packet.EndDraft);
      dispatch({
        type: ACTIONS.END_DRAFT,
        payload: {
          uuid: uuid2str(present(packet.EndDraft.uuid)),
          packet: wpacket
        }
      })
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
        dispatch({ type: ACTIONS.RESET, payload: null });
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

  const sendWebPacket = (wpacket: WebPacket) => {
    wsRef.current?.send(JSON.stringify(wpacket));
  }

  const getRecipient = () => {
    return present(sendFieldRef.current?.value);
  };
  const getTextboxContent = () => {
    return present(inputRef.current?.value);
  }

  const handleTypedDraft = (content: string) => {
    setInputContent(content);
  };

  useEffect(() => {
    if (!state.username) {
      return;
    }
    if (state.workingDraft) {
      if (state.currentDraft.uuid) {
        // edit this draft
        const packet: WebPacket = {
          destination: { User: getRecipient() },
          content: {
            Edit: {
              uuid: str2uuid(present(state.currentDraft.uuid)),
              content: inputContent
            }
          }
        }
        sendWebPacket(packet);
        dispatch({
          type: ACTIONS.UPDATE_CURRENT_DRAFT, 
          payload: (pstate: State) => (
            { ...pstate.currentDraft, content: inputContent }
          )
        });
      }
    } else {
      dispatch({type: ACTIONS.UPDATE_WORKING_DRAFT, payload: true});
      
      // start a draft
      const packet: WebPacket = {
        destination: { User: getRecipient() },
        content: {
          StartDraft: null
        }
      }
      dispatch({type: ACTIONS.UPDATE_CURRENT_DRAFT, payload: (pdraft: State) => ({
        ...pdraft.currentDraft,
        content: inputContent,
      })});
      sendWebPacket(packet);
    }
  }, [inputContent]);

  const sendDraft = (content: string | undefined, send_to: string | undefined) => {
    if (!content || !send_to) {
      return;
    }
    console.log('clicked sending draft!')
    dispatch({type: ACTIONS.SEND_DRAFT, payload: (pstate: State) => {
      const packet: WebPacket = {
        destination: { User: send_to },
        content: {
          EndDraft: {
            uuid: str2uuid(present(pstate.currentDraft.uuid)),
            content: content
          }
        }
      }
      sendWebPacket(packet);
      inputRef.current!.value = '';
    }});
  }

  return (
    <div>
      {isConnected ? (<div>
        <p> Connected! </p>
        Message room for {username}
        <br />
        <div>
          <Messages messages={state.messages} username={state.username} drafts={state.senderDrafts} />
          Send to: <input type="text" id='send_to' ref={sendFieldRef}/>
          Message: <input type="text" id='message_content' ref={inputRef} onInput={() => { handleTypedDraft(getTextboxContent()); }} />
          <button onClick={
            () => {
              sendDraft(inputRef.current?.value, sendFieldRef.current?.value);
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