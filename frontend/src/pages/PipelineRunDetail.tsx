import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, X } from 'lucide-react';
import {
  getPipelineRun,
  getPipelineStages,
  cancelPipelineRun,
  listPendingApprovals,
} from '@/api/pipelines';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { PipelineRunStatusBadge, PipelineStageStatusBadge } from '@/components/ui/badge';
import { formatDate, formatDuration } from '@/lib/utils';
import { ApprovalModal } from '@/components/pipelines/ApprovalModal';

export function PipelineRunDetail() {
  const { name, runId } = useParams<{ name: string; runId: string }>();
  const queryClient = useQueryClient();
  const [selectedApprovalId, setSelectedApprovalId] = useState<number | null>(null);

  const { data: run, isLoading: runLoading } = useQuery({
    queryKey: ['pipeline-run', runId],
    queryFn: () => getPipelineRun(Number(runId)),
    enabled: !!runId,
    refetchInterval: 3000, // Refresh every 3 seconds for live updates
  });

  const { data: stages = [], isLoading: stagesLoading } = useQuery({
    queryKey: ['pipeline-stages', runId],
    queryFn: () => getPipelineStages(Number(runId)),
    enabled: !!runId,
    refetchInterval: 3000,
  });

  const { data: approvals = [] } = useQuery({
    queryKey: ['approvals'],
    queryFn: listPendingApprovals,
    refetchInterval: 5000,
  });

  const cancelMutation = useMutation({
    mutationFn: () => cancelPipelineRun(Number(runId)),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipeline-run', runId] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-stages', runId] });
    },
  });

  const handleCancel = () => {
    if (window.confirm('Are you sure you want to cancel this pipeline run?')) {
      cancelMutation.mutate();
    }
  };

  const pendingApproval = approvals.find((a) => a.run_id === Number(runId));

  if (runLoading || stagesLoading) {
    return <div className="text-center py-12">Loading...</div>;
  }

  if (!run) {
    return (
      <div className="text-center py-12">
        <p className="mb-4">Pipeline run not found</p>
        <Link to={`/pipelines/${name}`}>
          <Button variant="outline">Back to Pipeline</Button>
        </Link>
      </div>
    );
  }

  const canCancel =
    run.status === 'Pending' ||
    run.status === 'Running' ||
    run.status === 'WaitingApproval';

  return (
    <div className="space-y-8">
      <div className="flex items-center gap-4">
        <Link to={`/pipelines/${name}`}>
          <Button variant="ghost" size="sm">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back
          </Button>
        </Link>
        <h1 className="text-3xl font-bold flex-1">
          Run #{run.id}
        </h1>
        <div className="flex items-center gap-2">
          <PipelineRunStatusBadge status={run.status} />
          {canCancel && (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleCancel}
              disabled={cancelMutation.isPending}
            >
              <X className="mr-2 h-4 w-4" />
              Cancel
            </Button>
          )}
        </div>
      </div>

      {/* Run Info */}
      <Card>
        <CardHeader>
          <CardTitle>Run Information</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <div className="text-sm text-muted-foreground">Status</div>
              <div className="mt-1">
                <PipelineRunStatusBadge status={run.status} />
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Trigger</div>
              <div className="mt-1 font-medium">
                {run.trigger_event || 'manual'}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Started</div>
              <div className="mt-1 font-medium">
                {run.started_at ? formatDate(run.started_at) : '-'}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Duration</div>
              <div className="mt-1 font-medium">
                {formatDuration(run.started_at, run.completed_at)}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Pending Approval Banner */}
      {pendingApproval && (
        <Card className="border-yellow-600 bg-yellow-50 dark:bg-yellow-950">
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold mb-1">Approval Required</h3>
                <p className="text-sm text-muted-foreground">
                  This pipeline is waiting for approval to continue.
                </p>
              </div>
              <Button onClick={() => setSelectedApprovalId(pendingApproval.id)}>
                Review Approval
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Stage DAG Visualization */}
      <Card>
        <CardHeader>
          <CardTitle>Pipeline Stages</CardTitle>
        </CardHeader>
        <CardContent>
          {stages.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No stages yet
            </div>
          ) : (
            <div className="space-y-4">
              {stages.map((stage, index) => (
                <div key={stage.id} className="relative">
                  {index > 0 && (
                    <div className="absolute left-6 -top-4 w-0.5 h-4 bg-border" />
                  )}
                  <div className="flex items-start gap-4 p-4 border rounded-lg">
                    <div className="flex-shrink-0 w-12 h-12 rounded-full border-2 flex items-center justify-center bg-background">
                      <div className="text-sm font-semibold">{index + 1}</div>
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3 mb-2">
                        <h3 className="font-semibold">{stage.stage_name}</h3>
                        <PipelineStageStatusBadge status={stage.status} />
                      </div>
                      <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm">
                        {stage.agent_id && (
                          <div>
                            <span className="text-muted-foreground">Agent: </span>
                            <span className="font-mono">{stage.agent_id}</span>
                          </div>
                        )}
                        {stage.started_at && (
                          <div>
                            <span className="text-muted-foreground">Started: </span>
                            {formatDate(stage.started_at)}
                          </div>
                        )}
                        <div>
                          <span className="text-muted-foreground">Duration: </span>
                          {formatDuration(stage.started_at, stage.completed_at)}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Approval Modal */}
      {selectedApprovalId && (
        <ApprovalModal
          approvalId={selectedApprovalId}
          onClose={() => setSelectedApprovalId(null)}
        />
      )}
    </div>
  );
}
