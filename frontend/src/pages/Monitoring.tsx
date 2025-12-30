import { useQuery } from '@tanstack/react-query';
import {
  getSystemHealth,
  getMetricsSnapshot,
  listAlerts,
  getPerformanceStats,
  getCostReports,
} from '@/api/monitoring';
import { HealthStatus } from '@/components/monitoring/HealthStatus';
import { MetricCard } from '@/components/monitoring/MetricCard';
import { AlertsList } from '@/components/monitoring/AlertsList';
import { CostChart } from '@/components/monitoring/CostChart';
import { PerformanceTable } from '@/components/monitoring/PerformanceTable';
import {
  Activity,
  Clock,
  TrendingUp,
  AlertTriangle,
  Zap,
} from 'lucide-react';

export function Monitoring() {
  // Fetch system health with auto-refresh every 30 seconds
  const { data: health } = useQuery({
    queryKey: ['systemHealth'],
    queryFn: getSystemHealth,
    refetchInterval: 30000,
  });

  // Fetch metrics snapshot with auto-refresh
  const { data: metrics } = useQuery({
    queryKey: ['metrics'],
    queryFn: getMetricsSnapshot,
    refetchInterval: 30000,
  });

  // Fetch active alerts with auto-refresh
  const { data: alertsData } = useQuery({
    queryKey: ['alerts'],
    queryFn: () => listAlerts({ status: 'Active', limit: 50 }),
    refetchInterval: 30000,
  });

  // Fetch performance stats
  const { data: performanceData } = useQuery({
    queryKey: ['performance'],
    queryFn: () => getPerformanceStats(),
    refetchInterval: 60000, // Refresh every minute
  });

  // Fetch cost reports
  const { data: costData } = useQuery({
    queryKey: ['costs'],
    queryFn: () => getCostReports({ period: 'monthly' }),
    refetchInterval: 60000,
  });

  const summary = metrics?.summary || {
    active_agents: 0,
    total_requests_24h: 0,
    avg_response_time_ms: 0,
    error_rate: 0,
    total_tokens_24h: 0,
  };

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Monitoring Dashboard</h1>
        <div className="text-sm text-muted-foreground">
          Auto-refresh: 30s
        </div>
      </div>

      {/* System Health */}
      {health && (
        <HealthStatus status={health.status} components={health.components} />
      )}

      {/* Key Metrics Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          label="Active Agents"
          value={summary.active_agents}
          icon={<Activity className="h-5 w-5 text-muted-foreground" />}
          variant="default"
        />
        <MetricCard
          label="Requests (24h)"
          value={summary.total_requests_24h.toLocaleString()}
          icon={<TrendingUp className="h-5 w-5 text-muted-foreground" />}
          variant="default"
        />
        <MetricCard
          label="Avg Response Time"
          value={`${summary.avg_response_time_ms.toFixed(0)}ms`}
          icon={<Clock className="h-5 w-5 text-muted-foreground" />}
          variant={
            summary.avg_response_time_ms > 1000
              ? 'warning'
              : summary.avg_response_time_ms > 2000
              ? 'danger'
              : 'success'
          }
        />
        <MetricCard
          label="Error Rate"
          value={`${(summary.error_rate * 100).toFixed(2)}%`}
          icon={<AlertTriangle className="h-5 w-5 text-muted-foreground" />}
          variant={
            summary.error_rate > 0.05
              ? 'danger'
              : summary.error_rate > 0.01
              ? 'warning'
              : 'success'
          }
        />
      </div>

      {/* Secondary Metrics */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <MetricCard
          label="Total Tokens (24h)"
          value={summary.total_tokens_24h.toLocaleString()}
          icon={<Zap className="h-5 w-5 text-muted-foreground" />}
          variant="default"
        />
        <MetricCard
          label="Active Alerts"
          value={health?.active_alerts || 0}
          icon={<AlertTriangle className="h-5 w-5 text-muted-foreground" />}
          variant={
            (health?.active_alerts || 0) > 5
              ? 'danger'
              : (health?.active_alerts || 0) > 0
              ? 'warning'
              : 'success'
          }
        />
      </div>

      {/* Alerts List */}
      {alertsData && <AlertsList alerts={alertsData.alerts} />}

      {/* Performance and Cost Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Performance Table - 2/3 width */}
        <div className="lg:col-span-2">
          {performanceData && (
            <PerformanceTable stats={performanceData.stats} />
          )}
        </div>

        {/* Cost Chart - 1/3 width */}
        <div className="lg:col-span-1">
          {costData && (
            <CostChart
              totalCost={costData.report.total_cost}
              breakdown={costData.report.breakdown_by_agent}
            />
          )}
        </div>
      </div>
    </div>
  );
}
