import React, { useEffect, useRef } from 'react';

const LogViewer = ({ logs = [] }) => {
  const logsEndRef = useRef(null);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="bg-gray-900 text-gray-200 p-4 rounded-lg shadow flex-1 overflow-y-auto font-mono text-sm">
      {logs.length === 0 && <p className="text-gray-500 italic">No logs yet...</p>}
      {logs.map((log, index) => (
        <div key={index} className={`mb-1 ${log.source === 'stderr' ? 'text-red-400' : 'text-green-400'}`}>
          <span className="text-gray-500 mr-2">[{new Date(log.timestamp * 1000).toLocaleTimeString()}]</span>
          <span>{log.message}</span>
        </div>
      ))}
      <div ref={logsEndRef} />
    </div>
  );
};

export default LogViewer;
