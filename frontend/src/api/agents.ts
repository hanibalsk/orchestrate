import { apiRequest } from './client';
import type { Agent, CreateAgentRequest, Message, SystemStatus } from './types';

export async function listAgents(): Promise<Agent[]> {
  return apiRequest<Agent[]>('/agents');
}

export async function getAgent(id: string): Promise<Agent> {
  return apiRequest<Agent>(`/agents/${id}`);
}

export async function createAgent(data: CreateAgentRequest): Promise<Agent> {
  return apiRequest<Agent>('/agents', {
    method: 'POST',
    body: data,
  });
}

export async function pauseAgent(id: string): Promise<Agent> {
  return apiRequest<Agent>(`/agents/${id}/pause`, { method: 'POST' });
}

export async function resumeAgent(id: string): Promise<Agent> {
  return apiRequest<Agent>(`/agents/${id}/resume`, { method: 'POST' });
}

export async function terminateAgent(id: string): Promise<Agent> {
  return apiRequest<Agent>(`/agents/${id}/terminate`, { method: 'POST' });
}

export async function getMessages(id: string): Promise<Message[]> {
  return apiRequest<Message[]>(`/agents/${id}/messages`);
}

export async function sendMessage(
  id: string,
  content: string
): Promise<void> {
  return apiRequest(`/agents/${id}/message`, {
    method: 'POST',
    body: { content },
  });
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return apiRequest<SystemStatus>('/status');
}
