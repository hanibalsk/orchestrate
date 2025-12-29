import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle2, XCircle, Clock, Play } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TestRun, TestRunStatus } from '@/api/test-types';

interface TestRunHistoryProps {
  runs?: TestRun[];
  isLoading?: boolean;
}

export function TestRunHistory({ runs, isLoading }: TestRunHistoryProps) {
  const getStatusIcon = (status: TestRunStatus) => {
    switch (status) {
      case 'completed':
        return <CheckCircle2 className="h-4 w-4 text-success" />;
      case 'failed':
        return <XCircle className="h-4 w-4 text-destructive" />;
      case 'running':
        return <Play className="h-4 w-4 text-info animate-pulse" />;
      default:
        return <Clock className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const getStatusBadge = (status: TestRunStatus) => {
    const variants: Record<
      TestRunStatus,
      'default' | 'secondary' | 'destructive'
    > = {
      completed: 'default',
      failed: 'destructive',
      running: 'default',
      pending: 'secondary',
    };

    return (
      <Badge variant={variants[status]} className="text-xs">
        {status}
      </Badge>
    );
  };

  const formatDuration = (seconds?: number) => {
    if (!seconds) return 'N/A';
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    const minutes = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${minutes}m ${secs}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Test Run History</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  if (!runs || runs.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Test Run History</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">
            No test runs yet
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Test Run History</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {runs.map((run) => (
            <div
              key={run.run_id}
              className="flex items-center justify-between p-3 rounded border hover:bg-accent"
            >
              <div className="flex items-center gap-3 flex-1">
                {getStatusIcon(run.status)}
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium capitalize">
                      {run.scope}
                    </span>
                    {run.target && (
                      <span className="text-xs text-muted-foreground">
                        ({run.target})
                      </span>
                    )}
                    {getStatusBadge(run.status)}
                    {run.with_coverage && (
                      <Badge variant="secondary" className="text-xs">
                        coverage
                      </Badge>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {formatTimestamp(run.started_at)}
                  </div>
                </div>
              </div>

              {run.results && (
                <div className="flex items-center gap-4 ml-4">
                  <div className="text-right">
                    <div className="flex items-center gap-2 text-sm">
                      <span className="text-success font-medium">
                        {run.results.passed}
                      </span>
                      <span className="text-muted-foreground">/</span>
                      <span className="text-destructive font-medium">
                        {run.results.failed}
                      </span>
                      <span className="text-muted-foreground">/</span>
                      <span className="text-muted-foreground">
                        {run.results.total}
                      </span>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {formatDuration(run.results.duration_secs)}
                    </div>
                  </div>
                  <div
                    className={cn(
                      'text-2xl font-bold',
                      run.results.failed === 0 ? 'text-success' : 'text-destructive'
                    )}
                  >
                    {run.results.failed === 0 ? '✓' : '✗'}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
