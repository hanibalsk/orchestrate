// Autonomous Processing Dashboard
// Epic 016: Autonomous Epic Processing - Story 16

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getAutoStatus,
  startAutoProcess,
  pauseAutoProcess,
  resumeAutoProcess,
  stopAutoProcess,
  listStuckAgents,
  listEdgeCases,
  listSessions,
  unblockSession,
  resolveEdgeCase,
  AutoProcessStatus,
  StuckAgent,
  EdgeCase,
  Session,
} from '@/api/autonomous';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

// State badge colors
function getStateBadge(state: string) {
  const variants: Record<string, { variant: 'default' | 'secondary' | 'destructive' | 'outline'; label: string }> = {
    idle: { variant: 'secondary', label: 'Idle' },
    analyzing: { variant: 'default', label: 'Analyzing' },
    discovering: { variant: 'default', label: 'Discovering' },
    planning: { variant: 'default', label: 'Planning' },
    executing: { variant: 'default', label: 'Executing' },
    reviewing: { variant: 'default', label: 'Reviewing' },
    pr_creation: { variant: 'default', label: 'Creating PR' },
    pr_monitoring: { variant: 'default', label: 'Monitoring PR' },
    pr_fixing: { variant: 'default', label: 'Fixing PR' },
    pr_merging: { variant: 'default', label: 'Merging' },
    completing: { variant: 'default', label: 'Completing' },
    done: { variant: 'outline', label: 'Done' },
    blocked: { variant: 'destructive', label: 'Blocked' },
    paused: { variant: 'secondary', label: 'Paused' },
  };
  const config = variants[state] || { variant: 'secondary' as const, label: state };
  return <Badge variant={config.variant}>{config.label}</Badge>;
}

function getSeverityBadge(severity: string) {
  const variants: Record<string, 'default' | 'secondary' | 'destructive' | 'outline'> = {
    low: 'secondary',
    medium: 'default',
    high: 'destructive',
    critical: 'destructive',
  };
  return <Badge variant={variants[severity] || 'secondary'}>{severity}</Badge>;
}

function getEdgeCaseTypeBadge(type: string) {
  const labels: Record<string, string> = {
    delayed_ci_review: 'Delayed CI',
    merge_conflict: 'Merge Conflict',
    flaky_test: 'Flaky Test',
    service_downtime: 'Service Down',
    dependency_failure: 'Dependency',
    review_ping_pong: 'Review Loop',
    context_overflow: 'Context Overflow',
    rate_limit: 'Rate Limit',
    timeout: 'Timeout',
    auth_error: 'Auth Error',
    network_error: 'Network Error',
    unknown: 'Unknown',
  };
  return <Badge variant="outline">{labels[type] || type}</Badge>;
}

// Processing Status Panel
function StatusPanel({ status }: { status: AutoProcessStatus }) {
  const isActive = status.state !== 'idle' && status.state !== 'done';
  const progressPercent = status.stories_completed + status.stories_failed > 0
    ? (status.stories_completed / (status.stories_completed + status.stories_failed)) * 100
    : 0;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Processing Status</CardTitle>
            <CardDescription>
              {status.session_id ? `Session: ${status.session_id.slice(0, 8)}...` : 'No active session'}
            </CardDescription>
          </div>
          {getStateBadge(status.state)}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {status.current_epic_id && (
          <div>
            <p className="text-sm text-muted-foreground">Current Epic</p>
            <p className="font-medium">{status.current_epic_id}</p>
          </div>
        )}
        {status.current_story_id && (
          <div>
            <p className="text-sm text-muted-foreground">Current Story</p>
            <p className="font-medium">{status.current_story_id}</p>
          </div>
        )}

        {isActive && (
          <>
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span>Progress</span>
                <span>{Math.round(progressPercent)}%</span>
              </div>
              <div className="w-full bg-muted rounded-full h-2">
                <div
                  className="bg-primary h-2 rounded-full transition-all"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="text-center p-3 bg-muted rounded-lg">
                <p className="text-2xl font-bold text-green-600">{status.stories_completed}</p>
                <p className="text-xs text-muted-foreground">Completed</p>
              </div>
              <div className="text-center p-3 bg-muted rounded-lg">
                <p className="text-2xl font-bold text-red-600">{status.stories_failed}</p>
                <p className="text-xs text-muted-foreground">Failed</p>
              </div>
            </div>
          </>
        )}

        <div className="grid grid-cols-3 gap-2 text-sm">
          <div>
            <span className="text-yellow-500 mr-1">*</span>
            <span>{status.agents_spawned} agents</span>
          </div>
          <div>
            <span className="text-blue-500 mr-1">*</span>
            <span>{status.queue_depth} queued</span>
          </div>
          <div>
            <span className="text-orange-500 mr-1">!</span>
            <span>{status.stuck_agents} stuck</span>
          </div>
        </div>

        <div className="text-sm text-muted-foreground">
          <span>Tokens: {status.tokens_used.toLocaleString()}</span>
          <span className="mx-2">|</span>
          <span>Success Rate: {(status.success_rate * 100).toFixed(1)}%</span>
        </div>
      </CardContent>
    </Card>
  );
}

// Control Panel
function ControlPanel({ status }: { status: AutoProcessStatus }) {
  const queryClient = useQueryClient();
  const [epicPattern, setEpicPattern] = useState('');
  const [showStartDialog, setShowStartDialog] = useState(false);

  const startMutation = useMutation({
    mutationFn: startAutoProcess,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autoStatus'] });
      setShowStartDialog(false);
    },
    onError: (error: Error) => {
      console.error('Failed to start autonomous processing:', error);
      alert(`Error: ${error.message || 'Failed to start autonomous processing'}`);
    },
  });

  const pauseMutation = useMutation({
    mutationFn: pauseAutoProcess,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['autoStatus'] }),
    onError: (error: Error) => {
      console.error('Failed to pause processing:', error);
      alert(`Error: ${error.message || 'Failed to pause processing'}`);
    },
  });

  const resumeMutation = useMutation({
    mutationFn: resumeAutoProcess,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['autoStatus'] }),
    onError: (error: Error) => {
      console.error('Failed to resume processing:', error);
      alert(`Error: ${error.message || 'Failed to resume processing'}`);
    },
  });

  const stopMutation = useMutation({
    mutationFn: stopAutoProcess,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['autoStatus'] }),
    onError: (error: Error) => {
      console.error('Failed to stop processing:', error);
      alert(`Error: ${error.message || 'Failed to stop processing'}`);
    },
  });

  const isActive = status.state !== 'idle' && status.state !== 'done';
  const isPaused = status.state === 'paused';
  const isBlocked = status.state === 'blocked';

  return (
    <Card>
      <CardHeader>
        <CardTitle>Controls</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {!isActive && (
          <Dialog open={showStartDialog} onOpenChange={setShowStartDialog}>
            <DialogTrigger asChild>
              <Button className="w-full" size="lg">
                Start Processing
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Start Autonomous Processing</DialogTitle>
                <DialogDescription>
                  Configure and start autonomous epic processing
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="epicPattern">Epic Pattern (optional)</Label>
                  <Input
                    id="epicPattern"
                    placeholder="e.g., epic-016-*"
                    value={epicPattern}
                    onChange={(e) => setEpicPattern(e.target.value)}
                  />
                </div>
              </div>
              <DialogFooter>
                <Button
                  onClick={() => startMutation.mutate({ epic_pattern: epicPattern || undefined })}
                  disabled={startMutation.isPending}
                >
                  {startMutation.isPending ? 'Starting...' : 'Start'}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        )}

        {isActive && !isPaused && (
          <Button
            variant="secondary"
            className="w-full"
            onClick={() => pauseMutation.mutate()}
            disabled={pauseMutation.isPending}
          >
            {pauseMutation.isPending ? 'Pausing...' : 'Pause'}
          </Button>
        )}

        {isPaused && (
          <Button
            className="w-full"
            onClick={() => resumeMutation.mutate()}
            disabled={resumeMutation.isPending}
          >
            {resumeMutation.isPending ? 'Resuming...' : 'Resume'}
          </Button>
        )}

        {isActive && (
          <Button
            variant="destructive"
            className="w-full"
            onClick={() => stopMutation.mutate()}
            disabled={stopMutation.isPending}
          >
            {stopMutation.isPending ? 'Stopping...' : 'Stop'}
          </Button>
        )}

        {isBlocked && (
          <div className="p-3 bg-red-50 dark:bg-red-950 rounded-lg">
            <p className="text-sm text-red-600 dark:text-red-400">
              Session is blocked. Check stuck agents or edge cases.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// Stuck Agents Panel
function StuckAgentsPanel({ stuckAgents }: { stuckAgents: StuckAgent[] }) {
  const queryClient = useQueryClient();
  const [selectedAgent, setSelectedAgent] = useState<StuckAgent | null>(null);
  const [action, setAction] = useState<'retry' | 'skip' | 'escalate'>('retry');
  const [showDialog, setShowDialog] = useState(false);

  const unblockMutation = useMutation({
    mutationFn: ({ id, action }: { id: string; action: 'retry' | 'skip' | 'escalate' }) =>
      unblockSession(id, { action }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['stuckAgents'] });
      queryClient.invalidateQueries({ queryKey: ['autoStatus'] });
      setShowDialog(false);
      setSelectedAgent(null);
    },
    onError: (error: Error) => {
      console.error('Failed to unblock session:', error);
      alert(`Error: ${error.message || 'Failed to unblock session'}`);
    },
  });

  if (stuckAgents.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Stuck Agents</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            No stuck agents - all systems operational
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Stuck Agents ({stuckAgents.length})</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {stuckAgents.map((agent) => (
            <div
              key={agent.id}
              className="flex items-center justify-between p-3 bg-muted rounded-lg"
            >
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <code className="text-xs">{agent.agent_id.slice(0, 8)}...</code>
                  <Badge variant="outline">{agent.stuck_type}</Badge>
                  {getSeverityBadge(agent.severity)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {new Date(agent.detected_at).toLocaleTimeString()}
                </p>
              </div>
              <Dialog open={showDialog && selectedAgent?.id === agent.id} onOpenChange={setShowDialog}>
                <DialogTrigger asChild>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setSelectedAgent(agent)}
                  >
                    Resolve
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Resolve Stuck Agent</DialogTitle>
                    <DialogDescription>
                      {agent.suggested_action}
                    </DialogDescription>
                  </DialogHeader>
                  <div className="space-y-4 py-4">
                    <div className="space-y-2">
                      <Label>Action</Label>
                      <Select
                        value={action}
                        onValueChange={(v) => setAction(v as typeof action)}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="retry">Retry</SelectItem>
                          <SelectItem value="skip">Skip</SelectItem>
                          <SelectItem value="escalate">Escalate</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                  <DialogFooter>
                    <Button
                      onClick={() =>
                        selectedAgent?.session_id &&
                        unblockMutation.mutate({
                          id: selectedAgent.session_id,
                          action,
                        })
                      }
                      disabled={unblockMutation.isPending || !selectedAgent?.session_id}
                    >
                      {unblockMutation.isPending ? 'Resolving...' : 'Apply'}
                    </Button>
                  </DialogFooter>
                </DialogContent>
              </Dialog>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// Edge Cases Panel
function EdgeCasesPanel({ edgeCases }: { edgeCases: EdgeCase[] }) {
  const queryClient = useQueryClient();

  const resolveMutation = useMutation({
    mutationFn: ({ id, resolution }: { id: number; resolution: 'auto_resolved' | 'manual_resolved' | 'bypassed' }) =>
      resolveEdgeCase(id, { resolution }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['edgeCases'] });
    },
    onError: (error: Error) => {
      console.error('Failed to resolve edge case:', error);
      alert(`Error: ${error.message || 'Failed to resolve edge case'}`);
    },
  });

  const unresolvedCases = edgeCases.filter(
    (e) => !['auto_resolved', 'manual_resolved', 'bypassed'].includes(e.resolution)
  );

  if (unresolvedCases.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Edge Cases</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            No unresolved edge cases
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Edge Cases ({unresolvedCases.length})</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {unresolvedCases.slice(0, 10).map((edgeCase) => (
            <div
              key={edgeCase.id}
              className="flex items-center justify-between p-3 bg-muted rounded-lg"
            >
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  {getEdgeCaseTypeBadge(edgeCase.edge_case_type)}
                  <Badge variant="secondary">{edgeCase.resolution}</Badge>
                  <span className="text-xs text-muted-foreground">
                    {edgeCase.retry_count} retries
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">
                  {new Date(edgeCase.detected_at).toLocaleTimeString()}
                </p>
              </div>
              <div className="flex gap-1">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    resolveMutation.mutate({
                      id: edgeCase.id,
                      resolution: 'manual_resolved',
                    })
                  }
                  disabled={resolveMutation.isPending}
                >
                  Resolve
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    resolveMutation.mutate({
                      id: edgeCase.id,
                      resolution: 'bypassed',
                    })
                  }
                  disabled={resolveMutation.isPending}
                >
                  Skip
                </Button>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// Sessions History Panel
function SessionsPanel({ sessions }: { sessions: Session[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Sessions</CardTitle>
      </CardHeader>
      <CardContent>
        {sessions.length === 0 ? (
          <p className="text-sm text-muted-foreground">No sessions yet</p>
        ) : (
          <div className="space-y-3">
            {sessions.slice(0, 5).map((session) => (
              <div key={session.id} className="p-3 bg-muted rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <code className="text-xs">{session.id.slice(0, 8)}...</code>
                  {getStateBadge(session.state)}
                </div>
                <div className="flex items-center gap-4 text-sm">
                  <span className="text-green-600">{session.stories_completed} done</span>
                  <span className="text-red-600">{session.stories_failed} failed</span>
                  <span className="text-muted-foreground">
                    {new Date(session.started_at).toLocaleDateString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// Main Dashboard Component
export function AutonomousProcessing() {
  const queryClient = useQueryClient();

  const { data: status, isLoading: statusLoading } = useQuery({
    queryKey: ['autoStatus'],
    queryFn: getAutoStatus,
    refetchInterval: 5000, // Refresh every 5 seconds
  });

  const { data: stuckAgents = [] } = useQuery({
    queryKey: ['stuckAgents'],
    queryFn: () => listStuckAgents(),
    refetchInterval: 10000,
  });

  const { data: edgeCases = [] } = useQuery({
    queryKey: ['edgeCases'],
    queryFn: () => listEdgeCases(undefined, 'unresolved'),
    refetchInterval: 10000,
  });

  const { data: sessions = [] } = useQuery({
    queryKey: ['sessions'],
    queryFn: () => listSessions(10),
    refetchInterval: 30000,
  });

  const defaultStatus: AutoProcessStatus = {
    session_id: null,
    state: 'idle',
    current_epic_id: null,
    current_story_id: null,
    stories_completed: 0,
    stories_failed: 0,
    agents_spawned: 0,
    tokens_used: 0,
    stuck_agents: 0,
    queue_depth: 0,
    success_rate: 0,
  };

  const currentStatus = status || defaultStatus;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Autonomous Processing</h1>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            // Invalidate only the relevant queries instead of all queries
            queryClient.invalidateQueries({ queryKey: ['autoStatus'] });
            queryClient.invalidateQueries({ queryKey: ['stuckAgents'] });
            queryClient.invalidateQueries({ queryKey: ['edgeCases'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
          }}
        >
          Refresh
        </Button>
      </div>

      {statusLoading ? (
        <div className="flex items-center justify-center p-8">
          <p className="text-muted-foreground">Loading...</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Main status and controls */}
          <div className="lg:col-span-2 space-y-6">
            <StatusPanel status={currentStatus} />
            <StuckAgentsPanel stuckAgents={stuckAgents} />
            <EdgeCasesPanel edgeCases={edgeCases} />
          </div>

          {/* Sidebar */}
          <div className="space-y-6">
            <ControlPanel status={currentStatus} />
            <SessionsPanel sessions={sessions} />
          </div>
        </div>
      )}
    </div>
  );
}
