import React, { createContext, useState, useEffect, useContext } from 'react';
import { listen } from '@tauri-apps/api/event';
import { startLocalServer, stopLocalServer, getServerStatus } from '../api/server';

const LocalServerContext = createContext();

export const LocalServerProvider = ({ children }) => {
  const [status, setStatus] = useState('STOPPED');
  const [logs, setLogs] = useState([]);
  const [config, setConfig] = useState({
    port: localStorage.getItem('kiro_server_port') || '7860',
  });

  useEffect(() => {
    localStorage.setItem('kiro_server_port', config.port);
  }, [config.port]);

  useEffect(() => {
    // Initial status check
    checkStatus();

    // Listen for status changes
    const unlistenStatus = listen('server-status-change', (event) => {
      setStatus(event.payload);
    });

    // Listen for logs
    const unlistenLogs = listen('server-log', (event) => {
      setLogs((prevLogs) => [...prevLogs, event.payload].slice(-1000)); // Keep last 1000 logs
    });

    // Auto-start on mount (if configured, logic to be added later)
    // For now, we just check status.
    // If we want auto-start by default as per spec:
    // startServer(); 

    return () => {
      unlistenStatus.then((f) => f());
      unlistenLogs.then((f) => f());
    };
  }, []);

  const checkStatus = async () => {
    try {
      const currentStatus = await getServerStatus();
      setStatus(currentStatus);
    } catch (error) {
      console.error('Failed to get server status:', error);
      setStatus('ERROR');
    }
  };

  const startServer = async () => {
    try {
      await startLocalServer();
    } catch (error) {
      console.error('Failed to start server:', error);
    }
  };

  const stopServer = async () => {
    try {
      await stopLocalServer();
    } catch (error) {
      console.error('Failed to stop server:', error);
    }
  };

  const clearLogs = () => {
    setLogs([]);
  };

  return (
    <LocalServerContext.Provider
      value={{
        status,
        logs,
        config,
        setConfig,
        startServer,
        stopServer,
        clearLogs,
      }}
    >
      {children}
    </LocalServerContext.Provider>
  );
};

export const useLocalServer = () => useContext(LocalServerContext);