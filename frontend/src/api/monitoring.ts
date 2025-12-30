import { apiRequest } from './client';
import type {
  SystemHealth,
  Alert,
  AlertStatus,
  AgentPerformance,
  CostReport,
  AcknowledgeAlertRequest,
  MetricValue,
  MetricsSummary,
} from './types';

// Response types matching backend
interface MetricsSnapshotResponse {
  timestamp: string;
  metrics: MetricValue[];
  summary: MetricsSummary;
}

interface AlertsListResponse {
  total: number;
  offset: number;
  limit: number;
  alerts: Alert[];
}

interface PerformanceResponse {
  period_start: string;
  period_end: string;
  stats: AgentPerformance[];
}

interface CostReportResponse {
  period: string;
  report: CostReport;
}

// GET /api/health - Get system health status
export async function getSystemHealth(): Promise<SystemHealth> {
  return apiRequest<SystemHealth>('/health');
}

// GET /api/metrics - Get current metrics snapshot
export async function getMetricsSnapshot(): Promise<MetricsSnapshotResponse> {
  return apiRequest<MetricsSnapshotResponse>('/metrics');
}

// GET /api/alerts - List alerts
export async function listAlerts(params?: {
  status?: AlertStatus;
  severity?: string;
  limit?: number;
  offset?: number;
}): Promise<AlertsListResponse> {
  const query = new URLSearchParams();
  if (params?.status) query.set('status', params.status);
  if (params?.severity) query.set('severity', params.severity);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());

  const queryString = query.toString();
  const endpoint = queryString ? `/alerts?${queryString}` : '/alerts';

  return apiRequest<AlertsListResponse>(endpoint);
}

// POST /api/alerts/:id/acknowledge - Acknowledge an alert
export async function acknowledgeAlert(
  id: number,
  request: AcknowledgeAlertRequest
): Promise<Alert> {
  return apiRequest<Alert>(`/alerts/${id}/acknowledge`, {
    method: 'POST',
    body: request,
  });
}

// GET /api/performance - Get agent performance stats
export async function getPerformanceStats(params?: {
  agent_type?: string;
  start?: string;
  end?: string;
}): Promise<PerformanceResponse> {
  const query = new URLSearchParams();
  if (params?.agent_type) query.set('agent_type', params.agent_type);
  if (params?.start) query.set('start', params.start);
  if (params?.end) query.set('end', params.end);

  const queryString = query.toString();
  const endpoint = queryString ? `/performance?${queryString}` : '/performance';

  return apiRequest<PerformanceResponse>(endpoint);
}

// GET /api/costs - Get cost reports
export async function getCostReports(params?: {
  period?: string;
  start?: string;
  end?: string;
  epic_id?: string;
  agent_type?: string;
}): Promise<CostReportResponse> {
  const query = new URLSearchParams();
  if (params?.period) query.set('period', params.period);
  if (params?.start) query.set('start', params.start);
  if (params?.end) query.set('end', params.end);
  if (params?.epic_id) query.set('epic_id', params.epic_id);
  if (params?.agent_type) query.set('agent_type', params.agent_type);

  const queryString = query.toString();
  const endpoint = queryString ? `/costs?${queryString}` : '/costs';

  return apiRequest<CostReportResponse>(endpoint);
}
