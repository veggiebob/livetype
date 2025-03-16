import React, { useEffect, useState } from 'react';
import { Draft } from './protocol';
import './DraftMessage.css';

interface DraftMessageProps {
  draft: Draft;
  sender: string;
}

const DraftMessage: React.FC<DraftMessageProps> = ({ draft, sender }) => {
  const [showCursor, setShowCursor] = useState(true);

  useEffect(() => {
    const cursorInterval = setInterval(() => {
      setShowCursor(prev => !prev);
    }, 500);

    return () => clearInterval(cursorInterval);
  }, []);
  
  return (
    <div className='received-draft'>
        {draft.content}{<span className="blinking-cursor">{showCursor && "|"}</span>}
    </div>
  );
};

export default DraftMessage;