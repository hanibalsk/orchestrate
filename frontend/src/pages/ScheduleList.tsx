import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { listSchedules } from '@/api/schedules';
import type { Schedule } from '@/api/types';
import { ScheduleTable } from '@/components/schedules/ScheduleTable';
import { CreateScheduleDialog } from '@/components/schedules/CreateScheduleDialog';
import { ScheduleRunsDialog } from '@/components/schedules/ScheduleRunsDialog';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

type FilterStatus = 'all' | 'enabled' | 'disabled';

export function ScheduleList() {
  const [statusFilter, setStatusFilter] = useState<FilterStatus>('all');
  const [selectedSchedule, setSelectedSchedule] = useState<Schedule | null>(
    null
  );
  const [historyDialogOpen, setHistoryDialogOpen] = useState(false);

  const { data: schedules = [], isLoading } = useQuery({
    queryKey: ['schedules'],
    queryFn: listSchedules,
    refetchInterval: 30000, // Refresh every 30 seconds for next_run updates
  });

  const filteredSchedules =
    statusFilter === 'all'
      ? schedules
      : schedules.filter((s) =>
          statusFilter === 'enabled' ? s.enabled : !s.enabled
        );

  const handleViewHistory = (schedule: Schedule) => {
    setSelectedSchedule(schedule);
    setHistoryDialogOpen(true);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Schedules</h1>
        <CreateScheduleDialog />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Schedules</CardTitle>
          <Select
            value={statusFilter}
            onValueChange={(value) => setStatusFilter(value as FilterStatus)}
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Schedules</SelectItem>
              <SelectItem value="enabled">Enabled Only</SelectItem>
              <SelectItem value="disabled">Disabled Only</SelectItem>
            </SelectContent>
          </Select>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-8 text-muted-foreground">
              Loading schedules...
            </div>
          ) : (
            <ScheduleTable
              schedules={filteredSchedules}
              onViewHistory={handleViewHistory}
            />
          )}
        </CardContent>
      </Card>

      <ScheduleRunsDialog
        schedule={selectedSchedule}
        open={historyDialogOpen}
        onOpenChange={setHistoryDialogOpen}
      />
    </div>
  );
}
