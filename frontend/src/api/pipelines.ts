import { apiRequest } from './client';
import type {
  Pipeline,
  PipelineRun,
  PipelineStage,
  ApprovalRequest,
  CreatePipelineRequest,
  UpdatePipelineRequest,
  TriggerRunRequest,
  ApprovalDecisionRequest,
} from './types';

// Pipeline CRUD
export async function listPipelines(): Promise<Pipeline[]> {
  return apiRequest<Pipeline[]>('/pipelines');
}

export async function getPipeline(name: string): Promise<Pipeline> {
  return apiRequest<Pipeline>(`/pipelines/${encodeURIComponent(name)}`);
}

export async function createPipeline(
  data: CreatePipelineRequest
): Promise<Pipeline> {
  return apiRequest<Pipeline>('/pipelines', {
    method: 'POST',
    body: data,
  });
}

export async function updatePipeline(
  name: string,
  data: UpdatePipelineRequest
): Promise<Pipeline> {
  return apiRequest<Pipeline>(`/pipelines/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: data,
  });
}

export async function deletePipeline(name: string): Promise<void> {
  return apiRequest<void>(`/pipelines/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

// Pipeline runs
export async function triggerPipelineRun(
  name: string,
  data?: TriggerRunRequest
): Promise<PipelineRun> {
  return apiRequest<PipelineRun>(
    `/pipelines/${encodeURIComponent(name)}/run`,
    {
      method: 'POST',
      body: data || {},
    }
  );
}

export async function listPipelineRuns(name: string): Promise<PipelineRun[]> {
  return apiRequest<PipelineRun[]>(
    `/pipelines/${encodeURIComponent(name)}/runs`
  );
}

export async function getPipelineRun(id: number): Promise<PipelineRun> {
  return apiRequest<PipelineRun>(`/pipeline-runs/${id}`);
}

export async function cancelPipelineRun(id: number): Promise<void> {
  return apiRequest<void>(`/pipeline-runs/${id}/cancel`, {
    method: 'POST',
  });
}

// Pipeline stages (helper to get stages for a run)
export async function getPipelineStages(runId: number): Promise<PipelineStage[]> {
  // This endpoint may need to be added to the backend
  // For now, we'll assume it exists or needs to be created
  return apiRequest<PipelineStage[]>(`/pipeline-runs/${runId}/stages`);
}

// Approvals
export async function listPendingApprovals(): Promise<ApprovalRequest[]> {
  return apiRequest<ApprovalRequest[]>('/approvals');
}

export async function approveApproval(
  id: number,
  data: ApprovalDecisionRequest
): Promise<ApprovalRequest> {
  return apiRequest<ApprovalRequest>(`/approvals/${id}/approve`, {
    method: 'POST',
    body: data,
  });
}

export async function rejectApproval(
  id: number,
  data: ApprovalDecisionRequest
): Promise<ApprovalRequest> {
  return apiRequest<ApprovalRequest>(`/approvals/${id}/reject`, {
    method: 'POST',
    body: data,
  });
}
