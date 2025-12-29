// Agent types matching Rust API
export type AgentType =
  // Development agents
  | 'story_developer'
  | 'code_reviewer'
  | 'issue_fixer'
  | 'explorer'
  // BMAD agents
  | 'bmad_orchestrator'
  | 'bmad_planner'
  // PR management
  | 'pr_shepherd'
  | 'pr_controller'
  | 'conflict_resolver'
  // System agents
  | 'background_controller'
  | 'scheduler';

export type AgentState =
  | 'created'
  | 'initializing'
  | 'running'
  | 'waiting_for_input'
  | 'waiting_for_external'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'terminated';

export interface Agent {
  id: string;
  agent_type: AgentType;
  state: AgentState;
  task: string;
  created_at: string;
  updated_at: string;
  error_message?: string;
}

export interface CreateAgentRequest {
  agent_type: AgentType;
  task: string;
  worktree_id?: string;
}

// Message types
export interface ToolCall {
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export interface ToolResult {
  tool_call_id: string;
  content: string;
  is_error: boolean;
}

export interface Message {
  id: number;
  role: 'user' | 'assistant' | 'tool' | 'system';
  content: string;
  created_at: string;
  tool_calls?: ToolCall[];
  tool_results?: ToolResult[];
}

// Status types
export interface SystemStatus {
  total_agents: number;
  running_agents: number;
  paused_agents: number;
  completed_agents: number;
}

// API response types
export interface ApiError {
  error: string;
  code?: string;
}

// WebSocket message types
export type WsMessageType =
  | 'agent_state'
  | 'agent_message'
  | 'system_status'
  | 'subscribe'
  | 'send_message';

export interface WsAgentStateMessage {
  type: 'agent_state';
  agent_id: string;
  state: AgentState;
}

export interface WsAgentMessage {
  type: 'agent_message';
  agent_id: string;
  role: string;
  content: string;
}

export interface WsSystemStatusMessage {
  type: 'system_status';
  total_agents: number;
  running_agents: number;
}

export type WsMessage =
  | WsAgentStateMessage
  | WsAgentMessage
  | WsSystemStatusMessage;

// Schedule types
export interface Schedule {
  id: number;
  name: string;
  cron_expression: string;
  agent_type: string;
  task: string;
  enabled: boolean;
  last_run: string | null;
  next_run: string | null;
  created_at: string;
}

export interface CreateScheduleRequest {
  name: string;
  cron_expression: string;
  agent_type: string;
  task: string;
  enabled?: boolean;
}

export interface UpdateScheduleRequest {
  name?: string;
  cron_expression?: string;
  agent_type?: string;
  task?: string;
  enabled?: boolean;
}

export type ScheduleRunStatus = 'running' | 'completed' | 'failed';

export interface ScheduleRun {
  id: number;
  schedule_id: number;
  agent_id: string | null;
  started_at: string;
  completed_at: string | null;
  status: ScheduleRunStatus;
  error_message: string | null;
}
