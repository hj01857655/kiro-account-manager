import { invoke } from '@tauri-apps/api/core';

export const startLocalServer = async () => {
  return await invoke('start_local_server');
};

export const stopLocalServer = async () => {
  return await invoke('stop_local_server');
};

export const getServerStatus = async () => {
  return await invoke('get_server_status');
};