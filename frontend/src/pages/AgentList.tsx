import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { listAgents } from '@/api/agents';
import type { AgentState } from '@/api/types';
import { AgentTable } from '@/components/agents/AgentTable';
import { CreateAgentDialog } from '@/components/agents/CreateAgentDialog';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

const stateOptions: { value: AgentState | 'all'; label: string }[] = [
  { value: 'all', label: 'All States' },
  { value: 'running', label: 'Running' },
  { value: 'paused', label: 'Paused' },
  { value: 'completed', label: 'Completed' },
  { value: 'failed', label: 'Failed' },
  { value: 'waiting_for_input', label: 'Waiting for Input' },
  { value: 'terminated', label: 'Terminated' },
];

export function AgentList() {
  const [stateFilter, setStateFilter] = useState<AgentState | 'all'>('all');

  const { data: agents = [], isLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: listAgents,
  });

  const filteredAgents =
    stateFilter === 'all'
      ? agents
      : agents.filter((a) => a.state === stateFilter);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Agents</h1>
        <CreateAgentDialog />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Agents</CardTitle>
          <Select
            value={stateFilter}
            onValueChange={(value) =>
              setStateFilter(value as AgentState | 'all')
            }
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {stateOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-8 text-muted-foreground">
              Loading agents...
            </div>
          ) : (
            <AgentTable agents={filteredAgents} />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
