// Test-related types matching the REST API from Story 10

export type TestType = 'unit' | 'integration' | 'e2e' | 'property';
export type Language = 'rust' | 'typescript' | 'python';
export type TestCategory = 'happy_path' | 'edge_case' | 'error_handling';
export type TestRunScope = 'all' | 'changed' | 'module';
export type TestRunStatus = 'pending' | 'running' | 'completed' | 'failed';
export type Priority = 'high' | 'medium' | 'low';
export type ChangeType = 'added' | 'modified' | 'deleted';

// Coverage types
export interface FileCoverage {
  file_path: string;
  lines_covered: number;
  lines_total: number;
  coverage_percent: number;
  functions_covered: number;
  functions_total: number;
  branches_covered: number;
  branches_total: number;
}

export interface ModuleCoverage {
  module_name: string;
  lines_covered: number;
  lines_total: number;
  coverage_percent: number;
  functions_covered: number;
  functions_total: number;
  branches_covered: number;
  branches_total: number;
  files: FileCoverage[];
}

export interface CoverageReport {
  timestamp: string;
  modules: ModuleCoverage[];
  overall_percent: number;
  overall_lines_covered: number;
  overall_lines_total: number;
}

export interface CoverageHistoryEntry {
  timestamp: string;
  overall_percent: number;
  module_coverage: Record<string, number>;
}

// Test generation types
export interface TestCase {
  name: string;
  category: TestCategory;
  code: string;
}

export interface GenerateTestsRequest {
  test_type: TestType;
  target?: string;
  language?: Language;
  story_id?: string;
  platform?: string;
}

export interface GenerateTestsResponse {
  test_cases: TestCase[];
  generated_count: number;
  target: string;
  test_type: TestType;
}

// Test run types
export interface TriggerTestRunRequest {
  scope: TestRunScope;
  target?: string;
  with_coverage: boolean;
}

export interface TestRunResults {
  passed: number;
  failed: number;
  skipped: number;
  total: number;
  duration_secs: number;
}

export interface TestRun {
  run_id: string;
  status: TestRunStatus;
  scope: TestRunScope;
  target?: string;
  with_coverage: boolean;
  started_at: string;
  completed_at?: string;
  results?: TestRunResults;
}

// Test suggestions types
export interface ChangedFunction {
  name: string;
  file_path: string;
  change_type: ChangeType;
  signature?: string;
}

export interface TestSuggestion {
  function: ChangedFunction;
  suggested_tests: string[];
  priority: Priority;
  reason: string;
}

export interface TestSuggestionsParams {
  pr_number?: number;
  branch?: string;
  priority?: Priority;
}
