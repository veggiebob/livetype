import React, { useEffect, useState } from 'react';
import { Draft } from './protocol';

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
    <div>
      <p><strong>{sender}</strong></p>
      <p>
        {draft.content}{showCursor && <span className="blinking-cursor">|</span>}
      </p>
    </div>
  );
};

export default DraftMessage;