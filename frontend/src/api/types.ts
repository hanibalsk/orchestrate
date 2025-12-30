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

// Pipeline types
export type PipelineRunStatus =
  | 'Pending'
  | 'Running'
  | 'WaitingApproval'
  | 'Succeeded'
  | 'Failed'
  | 'Cancelled';

export type PipelineStageStatus =
  | 'Pending'
  | 'Running'
  | 'WaitingApproval'
  | 'Succeeded'
  | 'Failed'
  | 'Skipped'
  | 'Cancelled';

export type ApprovalStatus = 'Pending' | 'Approved' | 'Rejected' | 'TimedOut';

export interface Pipeline {
  id: number;
  name: string;
  definition: string;
  enabled: boolean;
  created_at: string;
}

export interface PipelineRun {
  id: number;
  pipeline_id: number;
  status: PipelineRunStatus;
  trigger_event: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface PipelineStage {
  id: number;
  run_id: number;
  stage_name: string;
  status: PipelineStageStatus;
  agent_id: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface ApprovalRequest {
  id: number;
  stage_id: number;
  run_id: number;
  status: ApprovalStatus;
  required_approvers: string;
  required_count: number;
  approval_count: number;
  rejection_count: number;
  timeout_seconds: number | null;
  timeout_action: string | null;
  timeout_at: string | null;
  resolved_at: string | null;
  created_at: string;
}

export interface CreatePipelineRequest {
  name: string;
  definition: string;
  enabled?: boolean;
}

export interface UpdatePipelineRequest {
  definition?: string;
  enabled?: boolean;
}

export interface TriggerRunRequest {
  trigger_event?: string;
}

export interface ApprovalDecisionRequest {
  approver: string;
  comment?: string;
}

// Pipeline WebSocket message types
export interface WsPipelineRunMessage {
  type: 'pipeline_run_status';
  run_id: number;
  status: PipelineRunStatus;
}

export interface WsPipelineStageMessage {
  type: 'pipeline_stage_status';
  stage_id: number;
  run_id: number;
  stage_name: string;
  status: PipelineStageStatus;
}

export interface WsApprovalMessage {
  type: 'approval_request';
  approval_id: number;
  run_id: number;
  stage_name: string;
}

// Extend WsMessage type
export type WsMessageExtended =
  | WsMessage
  | WsPipelineRunMessage
  | WsPipelineStageMessage
  | WsApprovalMessage;

// Schedule types
export interface Schedule {
  id: number;
  name: string;
  cron_expression: string;
  agent_type: string;
  task: string;
  enabled: boolean;
  next_run_at: string | null;
  last_run_at: string | null;
  created_at: string;
  updated_at: string;
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
  status: string;
  trigger_type: string;
  agent_id: string | null;
  error_message: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

// Monitoring types
export type AlertSeverity = 'Info' | 'Warning' | 'Critical';
export type AlertStatus = 'Active' | 'Acknowledged' | 'Resolved';
export type HealthStatus = 'Healthy' | 'Degraded' | 'Unhealthy';

export interface Alert {
  id: number;
  rule_id: number;
  status: AlertStatus;
  severity: AlertSeverity;
  message: string;
  fingerprint: string;
  labels: Record<string, string>;
  triggered_at: string;
  resolved_at: string | null;
  acknowledged_at: string | null;
  acknowledged_by: string | null;
}

export interface MetricValue {
  name: string;
  value: number;
  labels: Record<string, string>;
  timestamp: string;
}

export interface MetricsSummary {
  active_agents: number;
  pending_prs?: number;
  queue_depth?: number;
  total_requests_24h?: number;
  avg_response_time_ms: number;
  error_rate?: number;
  error_rate_percent?: number;
  total_tokens_24h?: number;
  tokens_used_today?: number;
  cost_today_usd?: number;
}

export interface ComponentHealth {
  name: string;
  status: HealthStatus;
  message: string | null;
  last_check: string;
}

export interface SystemHealth {
  status: HealthStatus;
  components: ComponentHealth[];
  active_alerts: number;
  metrics_summary: MetricsSummary;
}

export interface AgentPerformance {
  agent_type: string;
  total_executions: number;
  successful_executions: number;
  failed_executions: number;
  avg_duration_seconds: number;
  success_rate: number;
}

export interface CostBreakdown {
  agent_type: string;
  total_cost: number;
  token_count: number;
}

export interface CostReport {
  period_start: string;
  period_end: string;
  // Frontend field name
  total_cost?: number;
  // Backend field name
  total_cost_usd?: number;
  // Frontend field name
  breakdown_by_agent?: CostBreakdown[];
  // Backend field name (object format)
  by_agent_type?: Record<string, { cost: number; tokens: number }>;
  breakdown_by_epic?: Record<string, number>;
  by_epic?: Record<string, number>;
}

export interface AcknowledgeAlertRequest {
  acknowledged_by: string;
  notes?: string;
}

// Deployment types
export type EnvironmentType = 'development' | 'staging' | 'production';
export type DeploymentStatus = 'pending' | 'in_progress' | 'success' | 'failed' | 'rolled_back';

export interface Environment {
  id: string;
  name: string;
  env_type: EnvironmentType;
  type: EnvironmentType; // alias for backwards compatibility
  url?: string;
  current_version?: string;
  last_deployment_at?: string;
  last_deployed_by?: string;
  is_protected: boolean;
  requires_approval: boolean;
  created_at: string;
  updated_at: string;
}

export interface Deployment {
  id: number;
  environment_id: string;
  environment_name: string;
  environment?: string;
  version: string;
  status: DeploymentStatus;
  strategy: string;
  provider?: string;
  deployed_by: string;
  started_at: string;
  completed_at?: string;
  rollback_version?: string;
  notes?: string;
  error_message?: string;
}

export interface CreateDeploymentRequest {
  environment_id?: string;
  environment?: string;
  version: string;
  strategy?: string;
  notes?: string;
}

export interface Release {
  id: number;
  version: string;
  tag_name?: string;
  title: string;
  description?: string;
  changelog?: string;
  created_by: string;
  created_at: string;
  is_prerelease: boolean;
  is_published: boolean;
  published?: boolean; // alias for backwards compatibility
  published_at?: string;
  download_url?: string;
  github_release_url?: string;
}

export interface CreateReleaseRequest {
  version: string;
  tag_name?: string;
  title?: string;
  description?: string;
  changelog?: string;
  is_prerelease?: boolean;
}
