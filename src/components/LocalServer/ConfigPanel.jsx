import React from 'react';

const ConfigPanel = ({ port, setPort, isRunning }) => {
  return (
    <div className="bg-white p-4 rounded-lg shadow mb-4">
      <h3 className="text-lg font-semibold mb-2">Configuration</h3>
      <div className="flex items-center space-x-4">
        <div className="flex-1">
          <label className="block text-sm font-medium text-gray-700">Port</label>
          <input
            type="number"
            value={port}
            onChange={(e) => setPort(e.target.value)}
            disabled={isRunning}
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm disabled:bg-gray-100"
            placeholder="7860"
          />
        </div>
        {/* Placeholder for future configs like Auto-start */}
      </div>
      <p className="text-xs text-gray-500 mt-2">
        {isRunning ? "Stop the server to change configuration." : "Port defaults to 7860."}
      </p>
    </div>
  );
};

export default ConfigPanel;
