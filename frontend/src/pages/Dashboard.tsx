import { useQuery } from '@tanstack/react-query';
import { listAgents, getSystemStatus } from '@/api/agents';
import { StatCard } from '@/components/agents/StatCard';
import { AgentTable } from '@/components/agents/AgentTable';
import { CreateAgentDialog } from '@/components/agents/CreateAgentDialog';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export function Dashboard() {
  const { data: status } = useQuery({
    queryKey: ['status'],
    queryFn: getSystemStatus,
    refetchInterval: 30000,
  });

  const { data: agents = [] } = useQuery({
    queryKey: ['agents'],
    queryFn: listAgents,
  });

  const recentAgents = agents.slice(0, 10);

  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold">Dashboard</h1>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Total Agents" value={status?.total_agents ?? 0} />
        <StatCard
          label="Running"
          value={status?.running_agents ?? 0}
          variant="success"
        />
        <StatCard
          label="Paused"
          value={status?.paused_agents ?? 0}
          variant="warning"
        />
        <StatCard
          label="Completed"
          value={status?.completed_agents ?? 0}
          variant="info"
        />
      </div>

      {/* Recent Agents */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Recent Agents</CardTitle>
          <CreateAgentDialog />
        </CardHeader>
        <CardContent>
          <AgentTable agents={recentAgents} />
        </CardContent>
      </Card>
    </div>
  );
}
