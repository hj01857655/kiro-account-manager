import { useLocalServer } from '../../contexts/LocalServerContext';
import LogViewer from './LogViewer';

export default function LocalServerPage() {
  const { status, logs, config, startServer, stopServer, clearLogs } = useLocalServer();
  const isRunning = status === 'RUNNING';

  return (
    <div className="p-6 h-full flex flex-col">
      <h1 className="text-2xl font-bold mb-6">Local Kiro 2API Server</h1>
      
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6 mb-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className={`w-3 h-3 rounded-full ${isRunning ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-lg font-semibold">
              {status}
            </span>
          </div>
          <button
            onClick={isRunning ? stopServer : startServer}
            className={`px-6 py-2 rounded font-medium ${
              isRunning
                ? 'bg-red-500 hover:bg-red-600 text-white'
                : 'bg-green-500 hover:bg-green-600 text-white'
            }`}
          >
            {isRunning ? 'Stop Server' : 'Start Server'}
          </button>
        </div>
        
        <div className="space-y-2 text-sm">
          <div>
            <span className="font-medium">Port:</span> {config.port}
          </div>
          <div>
            <span className="font-medium">URL:</span>{' '}
            {isRunning ? (
              <a
                href={`http://127.0.0.1:${config.port}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-500 hover:underline"
              >
                http://127.0.0.1:{config.port}
              </a>
            ) : (
              <span className="text-gray-400">Not running</span>
            )}
          </div>
        </div>
      </div>

      <div className="flex-1 bg-white dark:bg-gray-800 rounded-lg shadow p-6 overflow-hidden flex flex-col">
        <div className="flex justify-between items-center mb-2">
          <h3 className="text-lg font-semibold">Logs</h3>
          <button
            onClick={clearLogs}
            className="px-3 py-1 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300 dark:hover:bg-gray-600"
          >
            Clear
          </button>
        </div>
        <LogViewer logs={logs} />
      </div>
    </div>
  );
}
