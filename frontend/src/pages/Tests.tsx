import { useQuery } from '@tanstack/react-query';
import { getCoverageReport, getCoverageHistory } from '@/api/tests';
import { CoverageOverview } from '@/components/tests/CoverageOverview';
import { CoverageTrend } from '@/components/tests/CoverageTrend';
import { ModuleCoverageTable } from '@/components/tests/ModuleCoverageTable';
import { UntestedCodeList } from '@/components/tests/UntestedCodeList';
import { TestRunHistory } from '@/components/tests/TestRunHistory';
import { GenerateTestDialog } from '@/components/tests/GenerateTestDialog';

export function Tests() {
  const { data: coverageReport, isLoading: isLoadingCoverage } = useQuery({
    queryKey: ['coverage-report'],
    queryFn: () => getCoverageReport(),
    refetchInterval: 60000, // Refresh every minute
  });

  const { data: coverageHistory, isLoading: isLoadingHistory } = useQuery({
    queryKey: ['coverage-history'],
    queryFn: () => getCoverageHistory(30), // Last 30 entries
    refetchInterval: 60000,
  });

  // Mock test runs for now (will be replaced with actual API call)
  const { data: testRuns = [] } = useQuery({
    queryKey: ['test-runs'],
    queryFn: async () => [],
    refetchInterval: 10000,
  });

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Test Dashboard</h1>
        <GenerateTestDialog />
      </div>

      {/* Top Row: Coverage Overview and Trend */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <CoverageOverview
          report={coverageReport}
          isLoading={isLoadingCoverage}
        />
        <CoverageTrend history={coverageHistory} isLoading={isLoadingHistory} />
      </div>

      {/* Middle Row: Module Coverage */}
      <ModuleCoverageTable
        modules={coverageReport?.modules}
        isLoading={isLoadingCoverage}
      />

      {/* Bottom Row: Untested Code and Test Runs */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <UntestedCodeList
          modules={coverageReport?.modules}
          isLoading={isLoadingCoverage}
          threshold={50}
        />
        <TestRunHistory runs={testRuns} isLoading={false} />
      </div>
    </div>
  );
}
