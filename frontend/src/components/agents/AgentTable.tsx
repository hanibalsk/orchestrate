import { useNavigate } from 'react-router-dom';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { Agent } from '@/api/types';
import { pauseAgent, resumeAgent, terminateAgent } from '@/api/agents';
import { AgentStateBadge, AgentTypeBadge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { formatDate, truncate } from '@/lib/utils';
import { Pause, Play, XCircle } from 'lucide-react';

interface AgentTableProps {
  agents: Agent[];
  showActions?: boolean;
}

export function AgentTable({ agents, showActions = true }: AgentTableProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const pauseMutation = useMutation({
    mutationFn: pauseAgent,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
  });

  const resumeMutation = useMutation({
    mutationFn: resumeAgent,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
  });

  const terminateMutation = useMutation({
    mutationFn: terminateAgent,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
  });

  if (agents.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        No agents found
      </div>
    );
  }

  const canPause = (state: string) =>
    ['running', 'waiting_for_input', 'waiting_for_external'].includes(state);
  const canResume = (state: string) => state === 'paused';
  const canTerminate = (state: string) =>
    !['completed', 'failed', 'terminated'].includes(state);

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b">
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              ID
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Type
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              State
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Task
            </th>
            <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
              Created
            </th>
            {showActions && (
              <th className="text-left py-3 px-4 text-xs font-medium text-muted-foreground uppercase">
                Actions
              </th>
            )}
          </tr>
        </thead>
        <tbody>
          {agents.map((agent) => (
            <tr
              key={agent.id}
              className="border-b hover:bg-muted/50 cursor-pointer"
              onClick={() => navigate(`/agents/${agent.id}`)}
            >
              <td className="py-3 px-4 font-mono text-sm">
                {agent.id.slice(0, 8)}...
              </td>
              <td className="py-3 px-4">
                <AgentTypeBadge type={agent.agent_type} />
              </td>
              <td className="py-3 px-4">
                <AgentStateBadge state={agent.state} />
              </td>
              <td className="py-3 px-4 max-w-xs">
                <span title={agent.task}>{truncate(agent.task, 50)}</span>
              </td>
              <td className="py-3 px-4 text-sm text-muted-foreground">
                {formatDate(agent.created_at)}
              </td>
              {showActions && (
                <td className="py-3 px-4">
                  <div
                    className="flex gap-2"
                    onClick={(e) => e.stopPropagation()}
                  >
                    {canPause(agent.state) && (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => pauseMutation.mutate(agent.id)}
                        disabled={pauseMutation.isPending}
                      >
                        <Pause className="h-4 w-4" />
                      </Button>
                    )}
                    {canResume(agent.state) && (
                      <Button
                        size="sm"
                        variant="success"
                        onClick={() => resumeMutation.mutate(agent.id)}
                        disabled={resumeMutation.isPending}
                      >
                        <Play className="h-4 w-4" />
                      </Button>
                    )}
                    {canTerminate(agent.state) && (
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={() => terminateMutation.mutate(agent.id)}
                        disabled={terminateMutation.isPending}
                      >
                        <XCircle className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
