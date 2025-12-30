import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { AgentPerformance } from '@/api/types';
import { ArrowUpDown, ArrowUp, ArrowDown } from 'lucide-react';
import { cn } from '@/lib/utils';

interface PerformanceTableProps {
  stats: AgentPerformance[];
}

type SortField = 'agent_type' | 'total_executions' | 'success_rate' | 'avg_duration_seconds';
type SortDirection = 'asc' | 'desc';

export function PerformanceTable({ stats }: PerformanceTableProps) {
  const [sortField, setSortField] = useState<SortField>('total_executions');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

  // Handle undefined/null stats with default empty array
  const safeStats = stats ?? [];

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  const sortedStats = [...safeStats].sort((a, b) => {
    const aValue = a[sortField];
    const bValue = b[sortField];

    if (typeof aValue === 'string' && typeof bValue === 'string') {
      return sortDirection === 'asc'
        ? aValue.localeCompare(bValue)
        : bValue.localeCompare(aValue);
    }

    const aNum = Number(aValue);
    const bNum = Number(bValue);
    return sortDirection === 'asc' ? aNum - bNum : bNum - aNum;
  });

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <ArrowUpDown className="h-4 w-4" />;
    return sortDirection === 'asc' ? (
      <ArrowUp className="h-4 w-4" />
    ) : (
      <ArrowDown className="h-4 w-4" />
    );
  };

  if (safeStats.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Agent Performance</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8">
            <p className="text-sm text-muted-foreground">
              No performance data available
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Agent Performance</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b">
                <th
                  className="text-left p-3 cursor-pointer hover:bg-muted/50"
                  onClick={() => handleSort('agent_type')}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Agent Type</span>
                    <SortIcon field="agent_type" />
                  </div>
                </th>
                <th
                  className="text-right p-3 cursor-pointer hover:bg-muted/50"
                  onClick={() => handleSort('total_executions')}
                >
                  <div className="flex items-center justify-end gap-2">
                    <span className="text-sm font-medium">Total Runs</span>
                    <SortIcon field="total_executions" />
                  </div>
                </th>
                <th
                  className="text-right p-3 cursor-pointer hover:bg-muted/50"
                  onClick={() => handleSort('success_rate')}
                >
                  <div className="flex items-center justify-end gap-2">
                    <span className="text-sm font-medium">Success Rate</span>
                    <SortIcon field="success_rate" />
                  </div>
                </th>
                <th
                  className="text-right p-3 cursor-pointer hover:bg-muted/50"
                  onClick={() => handleSort('avg_duration_seconds')}
                >
                  <div className="flex items-center justify-end gap-2">
                    <span className="text-sm font-medium">Avg Duration</span>
                    <SortIcon field="avg_duration_seconds" />
                  </div>
                </th>
              </tr>
            </thead>
            <tbody>
              {sortedStats.map((stat) => {
                const successRatePercent = stat.success_rate * 100;
                const successRateColor =
                  successRatePercent >= 90
                    ? 'text-green-600'
                    : successRatePercent >= 70
                    ? 'text-yellow-600'
                    : 'text-red-600';

                return (
                  <tr key={stat.agent_type} className="border-b hover:bg-muted/50">
                    <td className="p-3">
                      <span className="text-sm font-medium capitalize">
                        {stat.agent_type.replace(/_/g, ' ')}
                      </span>
                    </td>
                    <td className="p-3 text-right text-sm">
                      {stat.total_executions}
                      <div className="text-xs text-muted-foreground">
                        {stat.successful_executions} success / {stat.failed_executions} failed
                      </div>
                    </td>
                    <td className="p-3 text-right">
                      <span className={cn('text-sm font-medium', successRateColor)}>
                        {successRatePercent.toFixed(1)}%
                      </span>
                    </td>
                    <td className="p-3 text-right text-sm">
                      {stat.avg_duration_seconds.toFixed(1)}s
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}
