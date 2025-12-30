// Autonomous Processing API Client
// Epic 016: Autonomous Epic Processing - Story 16

import { apiRequest } from './client';

// ==================== Types ====================

export interface AutoProcessConfig {
  max_agents?: number;
  max_retries?: number;
  dry_run?: boolean;
  auto_merge?: boolean;
  model?: string;
}

export interface StartAutoProcessRequest {
  epic_pattern?: string;
  config?: AutoProcessConfig;
}

export interface StartAutoProcessResponse {
  session_id: string;
  status: string;
  message: string;
}

export interface AutoProcessStatus {
  session_id: string | null;
  state: string;
  current_epic_id: string | null;
  current_story_id: string | null;
  stories_completed: number;
  stories_failed: number;
  agents_spawned: number;
  tokens_used: number;
  stuck_agents: number;
  queue_depth: number;
  success_rate: number;
}

export interface ActionResponse {
  success: boolean;
  message: string;
  session_id?: string;
}

export interface StuckAgent {
  id: number;
  agent_id: string;
  session_id: string | null;
  stuck_type: string;
  severity: string;
  details: Record<string, unknown>;
  detected_at: string;
  resolved: boolean;
  suggested_action: string;
}

export interface EdgeCase {
  id: number;
  session_id: string | null;
  agent_id: string | null;
  story_id: string | null;
  edge_case_type: string;
  resolution: string;
  action_taken: string | null;
  retry_count: number;
  error_message: string | null;
  detected_at: string;
  resolved_at: string | null;
}

export interface Session {
  id: string;
  state: string;
  current_epic_id: string | null;
  current_story_id: string | null;
  started_at: string;
  completed_at: string | null;
  completed_count: number;
  failed_count: number;
  stories_completed: number;
  stories_failed: number;
  tokens_used: number;
}

export interface SessionMetrics {
  session_id: string;
  stories_completed: number;
  stories_failed: number;
  reviews_passed: number;
  reviews_failed: number;
  total_iterations: number;
  agents_spawned: number;
  tokens_used: number;
  success_rate: number;
  review_pass_rate: number;
  edge_cases_count: number;
  stuck_detections_count: number;
}

export interface ResolveEdgeCaseRequest {
  resolution: 'auto_resolved' | 'manual_resolved' | 'bypassed';
  notes?: string;
}

export interface UnblockRequest {
  action: 'retry' | 'skip' | 'escalate';
  notes?: string;
}

// ==================== API Functions ====================

export async function startAutoProcess(
  request: StartAutoProcessRequest
): Promise<StartAutoProcessResponse> {
  return apiRequest<StartAutoProcessResponse>('/epic/auto-process', {
    method: 'POST',
    body: request,
  });
}

export async function getAutoStatus(): Promise<AutoProcessStatus> {
  return apiRequest<AutoProcessStatus>('/epic/auto-status');
}

export async function pauseAutoProcess(): Promise<ActionResponse> {
  return apiRequest<ActionResponse>('/epic/auto-pause', { method: 'POST' });
}

export async function resumeAutoProcess(): Promise<ActionResponse> {
  return apiRequest<ActionResponse>('/epic/auto-resume', { method: 'POST' });
}

export async function stopAutoProcess(): Promise<ActionResponse> {
  return apiRequest<ActionResponse>('/epic/auto-stop', { method: 'POST' });
}

export async function listStuckAgents(sessionId?: string): Promise<StuckAgent[]> {
  const params = sessionId ? `?session_id=${sessionId}` : '';
  return apiRequest<StuckAgent[]>(`/epic/stuck-agents${params}`);
}

export async function unblockSession(
  id: string,
  request: UnblockRequest
): Promise<ActionResponse> {
  return apiRequest<ActionResponse>(`/epic/${id}/unblock`, {
    method: 'POST',
    body: request,
  });
}

export async function listEdgeCases(
  sessionId?: string,
  status?: string
): Promise<EdgeCase[]> {
  const params = new URLSearchParams();
  if (sessionId) params.append('session_id', sessionId);
  if (status) params.append('status', status);
  const queryString = params.toString();
  return apiRequest<EdgeCase[]>(`/epic/edge-cases${queryString ? `?${queryString}` : ''}`);
}

export async function resolveEdgeCase(
  id: number,
  request: ResolveEdgeCaseRequest
): Promise<ActionResponse> {
  return apiRequest<ActionResponse>(`/epic/edge-cases/${id}/resolve`, {
    method: 'POST',
    body: request,
  });
}

export async function listSessions(
  limit?: number,
  status?: string
): Promise<Session[]> {
  const params = new URLSearchParams();
  if (limit) params.append('limit', limit.toString());
  if (status) params.append('status', status);
  const queryString = params.toString();
  return apiRequest<Session[]>(`/epic/sessions${queryString ? `?${queryString}` : ''}`);
}

export async function getSession(id: string): Promise<Session> {
  return apiRequest<Session>(`/epic/sessions/${id}`);
}

export async function getSessionMetrics(id: string): Promise<SessionMetrics> {
  return apiRequest<SessionMetrics>(`/epic/sessions/${id}/metrics`);
}
