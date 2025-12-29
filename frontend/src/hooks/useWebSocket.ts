import { useEffect, useRef, useCallback } from 'react';
import { create } from 'zustand';
import type { AgentState, WsMessage } from '@/api/types';

interface WebSocketStore {
  connected: boolean;
  reconnecting: boolean;
  setConnected: (connected: boolean) => void;
  setReconnecting: (reconnecting: boolean) => void;
}

export const useWebSocketStore = create<WebSocketStore>((set) => ({
  connected: false,
  reconnecting: false,
  setConnected: (connected) => set({ connected }),
  setReconnecting: (reconnecting) => set({ reconnecting }),
}));

interface UseWebSocketOptions {
  agentId?: string;
  onAgentStateChange?: (agentId: string, state: AgentState) => void;
  onNewMessage?: (agentId: string, role: string, content: string) => void;
  onSystemStatus?: (total: number, running: number) => void;
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const ws = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const maxReconnectAttempts = 5;
  const { setConnected, setReconnecting } = useWebSocketStore();

  const handleMessage = useCallback(
    (data: WsMessage) => {
      switch (data.type) {
        case 'agent_state':
          options.onAgentStateChange?.(data.agent_id, data.state);
          break;
        case 'agent_message':
          options.onNewMessage?.(data.agent_id, data.role, data.content);
          break;
        case 'system_status':
          options.onSystemStatus?.(data.total_agents, data.running_agents);
          break;
      }
    },
    [options]
  );

  const connect = useCallback(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    ws.current = new WebSocket(wsUrl);

    ws.current.onopen = () => {
      setConnected(true);
      setReconnecting(false);
      reconnectAttempts.current = 0;

      // Subscribe to specific agent if provided
      if (options.agentId && ws.current) {
        ws.current.send(
          JSON.stringify({
            type: 'subscribe',
            channels: [`agent:${options.agentId}`],
          })
        );
      }
    };

    ws.current.onmessage = (event) => {
      try {
        const data: WsMessage = JSON.parse(event.data);
        handleMessage(data);
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

    ws.current.onclose = () => {
      setConnected(false);
      attemptReconnect();
    };

    ws.current.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }, [options.agentId, handleMessage, setConnected, setReconnecting]);

  const attemptReconnect = useCallback(() => {
    if (reconnectAttempts.current < maxReconnectAttempts) {
      setReconnecting(true);
      reconnectAttempts.current++;
      const delay = Math.min(
        1000 * Math.pow(2, reconnectAttempts.current),
        30000
      );
      setTimeout(connect, delay);
    }
  }, [connect, setReconnecting]);

  const sendMessage = useCallback((agentId: string, content: string) => {
    if (ws.current?.readyState === WebSocket.OPEN) {
      ws.current.send(
        JSON.stringify({
          type: 'send_message',
          agent_id: agentId,
          content,
        })
      );
    }
  }, []);

  useEffect(() => {
    connect();
    return () => {
      ws.current?.close();
    };
  }, [connect]);

  return { sendMessage };
}
