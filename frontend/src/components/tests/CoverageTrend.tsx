import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { CoverageHistoryEntry } from '@/api/test-types';

interface CoverageTrendProps {
  history?: CoverageHistoryEntry[];
  isLoading?: boolean;
}

export function CoverageTrend({ history, isLoading }: CoverageTrendProps) {
  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Coverage Trend</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  if (!history || history.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Coverage Trend</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">
            No historical data available
          </div>
        </CardContent>
      </Card>
    );
  }

  // Simple ASCII-style chart
  const maxPercent = Math.max(...history.map((h) => h.overall_percent));
  const minPercent = Math.min(...history.map((h) => h.overall_percent));
  const range = maxPercent - minPercent || 1;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Coverage Trend</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {/* Simple trend visualization */}
          <div className="h-48 flex items-end justify-between gap-1">
            {history.map((entry, index) => {
              const height = ((entry.overall_percent - minPercent) / range) * 100;
              return (
                <div
                  key={entry.timestamp}
                  className="flex-1 flex flex-col items-center"
                >
                  <div className="w-full flex flex-col items-center justify-end h-full">
                    <div
                      className="w-full bg-primary rounded-t transition-all hover:bg-primary/80"
                      style={{ height: `${Math.max(height, 5)}%` }}
                      title={`${entry.overall_percent.toFixed(1)}%`}
                    />
                  </div>
                  {index % Math.ceil(history.length / 5) === 0 && (
                    <div className="text-xs text-muted-foreground mt-1 rotate-45 origin-left whitespace-nowrap">
                      {new Date(entry.timestamp).toLocaleDateString()}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Stats */}
          <div className="flex justify-between text-sm">
            <div>
              <span className="text-muted-foreground">Min: </span>
              <span className="font-medium">{minPercent.toFixed(1)}%</span>
            </div>
            <div>
              <span className="text-muted-foreground">Max: </span>
              <span className="font-medium">{maxPercent.toFixed(1)}%</span>
            </div>
            <div>
              <span className="text-muted-foreground">Current: </span>
              <span className="font-medium">
                {history[history.length - 1].overall_percent.toFixed(1)}%
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
