import { useState, useEffect } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { Schedule } from '@/api/types';
import {
  pauseSchedule,
  resumeSchedule,
  runSchedule,
  deleteSchedule,
} from '@/api/schedules';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { truncate } from '@/lib/utils';
import { getTimeUntil, formatDateTime } from '@/lib/time';
import { Pause, Play, PlayCircle, Trash2 } from 'lucide-react';

interface ScheduleTableProps {
  schedules: Schedule[];
  onViewHistory?: (schedule: Schedule) => void;
}

export function ScheduleTable({
  schedules,
  onViewHistory,
}: ScheduleTableProps) {
  const queryClient = useQueryClient();
  const [, setNow] = useState(new Date());

  // Update time every second for countdown
  useEffect(() => {
    const interval = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const pauseMutation = useMutation({
    mutationFn: pauseSchedule,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['schedules'] }),
  });

  const resumeMutation = useMutation({
    mutationFn: resumeSchedule,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['schedules'] }),
  });

  const runMutation = useMutation({
    mutationFn: runSchedule,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['schedules'] });
      queryClient.invalidateQueries({ queryKey: ['agents'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteSchedule,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['schedules'] }),
  });

  if (schedules.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        No schedules found
      </div>
    );
  }

  const handleDelete = (id: number, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm('Are you sure you want to delete this schedule?')) {
      deleteMutation.mutate(id);
    }
  };

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b">
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Name
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Cron
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Agent
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Task
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Status
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Next Run
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          {schedules.map((schedule) => (
            <tr
              key={schedule.id}
              className="border-b hover:bg-muted/50 cursor-pointer"
              onClick={() => onViewHistory?.(schedule)}
            >
              <td className="py-3 px-4 font-medium">{schedule.name}</td>
              <td className="py-3 px-4 font-mono text-sm">
                {schedule.cron_expression}
              </td>
              <td className="py-3 px-4">
                <Badge variant="outline">{schedule.agent_type}</Badge>
              </td>
              <td className="py-3 px-4 max-w-xs">
                <span title={schedule.task}>{truncate(schedule.task, 40)}</span>
              </td>
              <td className="py-3 px-4">
                {schedule.enabled ? (
                  <Badge variant="success">Enabled</Badge>
                ) : (
                  <Badge variant="secondary">Disabled</Badge>
                )}
              </td>
              <td className="py-3 px-4">
                <div className="flex flex-col">
                  <span className="text-sm font-medium">
                    {getTimeUntil(schedule.next_run_at)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {formatDateTime(schedule.next_run_at)}
                  </span>
                </div>
              </td>
              <td className="py-3 px-4">
                <div
                  className="flex gap-2"
                  onClick={(e) => e.stopPropagation()}
                >
                  {schedule.enabled ? (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => pauseMutation.mutate(schedule.id)}
                      disabled={pauseMutation.isPending}
                      title="Pause schedule"
                    >
                      <Pause className="h-4 w-4" />
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      variant="success"
                      onClick={() => resumeMutation.mutate(schedule.id)}
                      disabled={resumeMutation.isPending}
                      title="Resume schedule"
                    >
                      <Play className="h-4 w-4" />
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => runMutation.mutate(schedule.id)}
                    disabled={runMutation.isPending}
                    title="Run now"
                  >
                    <PlayCircle className="h-4 w-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    onClick={(e) => handleDelete(schedule.id, e)}
                    disabled={deleteMutation.isPending}
                    title="Delete schedule"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
