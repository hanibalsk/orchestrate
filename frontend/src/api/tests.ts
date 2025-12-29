import { apiRequest } from './client';
import type {
  CoverageReport,
  CoverageHistoryEntry,
  GenerateTestsRequest,
  GenerateTestsResponse,
  TriggerTestRunRequest,
  TestRun,
  TestSuggestion,
  TestSuggestionsParams,
} from './test-types';

// Coverage API
export async function getCoverageReport(
  module?: string,
  diff?: boolean
): Promise<CoverageReport> {
  const params = new URLSearchParams();
  if (module) params.append('module', module);
  if (diff) params.append('diff', 'true');

  const query = params.toString();
  return apiRequest<CoverageReport>(
    `/tests/coverage${query ? `?${query}` : ''}`
  );
}

export async function getCoverageHistory(
  limit?: number,
  module?: string
): Promise<CoverageHistoryEntry[]> {
  const params = new URLSearchParams();
  if (limit) params.append('limit', limit.toString());
  if (module) params.append('module', module);

  const query = params.toString();
  return apiRequest<CoverageHistoryEntry[]>(
    `/tests/coverage/history${query ? `?${query}` : ''}`
  );
}

// Test generation API
export async function generateTests(
  request: GenerateTestsRequest
): Promise<GenerateTestsResponse> {
  return apiRequest<GenerateTestsResponse>('/tests/generate', {
    method: 'POST',
    body: request,
  });
}

// Test run API
export async function triggerTestRun(
  request: TriggerTestRunRequest
): Promise<TestRun> {
  return apiRequest<TestRun>('/tests/run', {
    method: 'POST',
    body: request,
  });
}

export async function getTestRun(
  runId: string,
  includeDetails?: boolean
): Promise<TestRun> {
  const params = new URLSearchParams();
  if (includeDetails) params.append('include_details', 'true');

  const query = params.toString();
  return apiRequest<TestRun>(
    `/tests/runs/${runId}${query ? `?${query}` : ''}`
  );
}

// Test suggestions API
export async function getTestSuggestions(
  params: TestSuggestionsParams
): Promise<TestSuggestion[]> {
  const queryParams = new URLSearchParams();
  if (params.pr_number)
    queryParams.append('pr_number', params.pr_number.toString());
  if (params.branch) queryParams.append('branch', params.branch);
  if (params.priority) queryParams.append('priority', params.priority);

  const query = queryParams.toString();
  return apiRequest<TestSuggestion[]>(
    `/tests/suggestions${query ? `?${query}` : ''}`
  );
}
