import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { CoverageReport } from '@/api/test-types';

interface CoverageOverviewProps {
  report?: CoverageReport;
  isLoading?: boolean;
}

export function CoverageOverview({ report, isLoading }: CoverageOverviewProps) {
  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Coverage Overview</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  if (!report) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Coverage Overview</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">
            No coverage data available
          </div>
        </CardContent>
      </Card>
    );
  }

  const getCoverageColor = (percent: number) => {
    if (percent >= 80) return 'text-success';
    if (percent >= 50) return 'text-warning';
    return 'text-destructive';
  };

  const getCoverageBgColor = (percent: number) => {
    if (percent >= 80) return 'bg-success/10';
    if (percent >= 50) return 'bg-warning/10';
    return 'bg-destructive/10';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Coverage Overview</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Overall Coverage */}
          <div
            className={cn(
              'p-6 rounded-lg text-center',
              getCoverageBgColor(report.overall_percent)
            )}
          >
            <div
              className={cn(
                'text-5xl font-bold',
                getCoverageColor(report.overall_percent)
              )}
            >
              {report.overall_percent.toFixed(1)}%
            </div>
            <div className="text-sm text-muted-foreground mt-2">
              Overall Coverage
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              {report.overall_lines_covered} / {report.overall_lines_total}{' '}
              lines covered
            </div>
          </div>

          {/* Module Breakdown */}
          <div className="space-y-2">
            <div className="text-sm font-medium">Top Modules</div>
            {report.modules.slice(0, 5).map((module) => (
              <div
                key={module.module_name}
                className="flex items-center justify-between p-2 rounded hover:bg-accent"
              >
                <div className="text-sm">{module.module_name}</div>
                <div
                  className={cn(
                    'text-sm font-medium',
                    getCoverageColor(module.coverage_percent)
                  )}
                >
                  {module.coverage_percent.toFixed(1)}%
                </div>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
