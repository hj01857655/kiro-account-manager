import React from 'react';
import { useLocalServer } from '../contexts/LocalServerContext';
import ConfigPanel from '../components/LocalServer/ConfigPanel';
import LogViewer from '../components/LocalServer/LogViewer';

const LocalServerPage = () => {
  const { status, logs, config, setConfig, startServer, stopServer, clearLogs } = useLocalServer();

  const isRunning = status === 'RUNNING' || status === 'STARTING';

  const handlePortChange = (newPort) => {
    setConfig({ ...config, port: newPort });
  };

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-gray-800">Local Kiro 2API Service</h1>
        <div className="flex items-center space-x-4">
          <div className="flex items-center">
             <span className={`h-3 w-3 rounded-full mr-2 ${status === 'RUNNING' ? 'bg-green-500' : status === 'ERROR' ? 'bg-red-500' : 'bg-gray-400'}`}></span>
             <span className="text-sm font-medium text-gray-600">{status}</span>
          </div>
          {isRunning ? (
            <button
              onClick={stopServer}
              className="bg-red-600 hover:bg-red-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
            >
              Stop Server
            </button>
          ) : (
            <button
              onClick={startServer}
              className="bg-green-600 hover:bg-green-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
            >
              Start Server
            </button>
          )}
        </div>
      </div>

      <ConfigPanel port={config.port} setPort={handlePortChange} isRunning={isRunning} />

      <div className="mt-6">
        <div className="flex justify-between items-center mb-2">
           <h3 className="text-lg font-semibold">Logs</h3>
           <button onClick={clearLogs} className="text-xs text-blue-600 hover:underline">Clear Logs</button>
        </div>
        <LogViewer logs={logs} />
      </div>
    </div>
  );
};

export default LocalServerPage;
