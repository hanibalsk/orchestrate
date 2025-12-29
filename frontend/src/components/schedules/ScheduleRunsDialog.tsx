import { useQuery } from '@tanstack/react-query';
import { getScheduleRuns } from '@/api/schedules';
import type { Schedule, ScheduleRun } from '@/api/types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { formatDateTime, formatDuration } from '@/lib/time';
import { CheckCircle, XCircle, Clock } from 'lucide-react';

interface ScheduleRunsDialogProps {
  schedule: Schedule | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function ScheduleRunStatusBadge({ status }: { status: string }) {
  const variants = {
    running: { variant: 'secondary' as const, icon: Clock },
    completed: { variant: 'success' as const, icon: CheckCircle },
    failed: { variant: 'destructive' as const, icon: XCircle },
  };

  const config = variants[status as keyof typeof variants] || variants.running;
  const Icon = config.icon;

  return (
    <Badge variant={config.variant}>
      <Icon className="h-3 w-3 mr-1" />
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </Badge>
  );
}

function calculateDuration(startedAt: string, completedAt: string | null): string {
  if (!completedAt) {
    const start = new Date(startedAt);
    const now = new Date();
    const seconds = Math.floor((now.getTime() - start.getTime()) / 1000);
    return formatDuration(seconds) + ' (running)';
  }

  const start = new Date(startedAt);
  const end = new Date(completedAt);
  const seconds = Math.floor((end.getTime() - start.getTime()) / 1000);
  return formatDuration(seconds);
}

export function ScheduleRunsDialog({
  schedule,
  open,
  onOpenChange,
}: ScheduleRunsDialogProps) {
  const { data: runs = [], isLoading } = useQuery({
    queryKey: ['schedule-runs', schedule?.id],
    queryFn: () => getScheduleRuns(schedule!.id),
    enabled: !!schedule && open,
  });

  if (!schedule) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Execution History: {schedule.name}</DialogTitle>
          <DialogDescription>
            Recent executions of this scheduled task
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="text-center py-8 text-muted-foreground">
            Loading execution history...
          </div>
        ) : runs.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No executions yet
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b">
                  <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                    Started
                  </th>
                  <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                    Duration
                  </th>
                  <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                    Status
                  </th>
                  <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                    Agent ID
                  </th>
                  <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                    Error
                  </th>
                </tr>
              </thead>
              <tbody>
                {runs.map((run: ScheduleRun) => (
                  <tr key={run.id} className="border-b hover:bg-muted/50">
                    <td className="py-3 px-4 text-sm">
                      {formatDateTime(run.started_at)}
                    </td>
                    <td className="py-3 px-4 text-sm">
                      {calculateDuration(run.started_at, run.completed_at)}
                    </td>
                    <td className="py-3 px-4">
                      <ScheduleRunStatusBadge status={run.status} />
                    </td>
                    <td className="py-3 px-4 font-mono text-sm">
                      {run.agent_id ? (
                        <a
                          href={`/agents/${run.agent_id}`}
                          className="text-primary hover:underline"
                          onClick={(e) => e.stopPropagation()}
                        >
                          {run.agent_id.slice(0, 8)}...
                        </a>
                      ) : (
                        <span className="text-muted-foreground">N/A</span>
                      )}
                    </td>
                    <td className="py-3 px-4 text-sm max-w-xs">
                      {run.error_message ? (
                        <span
                          className="text-destructive"
                          title={run.error_message}
                        >
                          {run.error_message.length > 50
                            ? run.error_message.slice(0, 50) + '...'
                            : run.error_message}
                        </span>
                      ) : (
                        <span className="text-muted-foreground">-</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
