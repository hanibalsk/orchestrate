import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Edit2, Play, Save, X } from 'lucide-react';
import {
  getPipeline,
  updatePipeline,
  triggerPipelineRun,
  listPipelineRuns,
} from '@/api/pipelines';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { PipelineRunStatusBadge } from '@/components/ui/badge';
import { formatDate } from '@/lib/utils';

export function PipelineDetail() {
  const { name } = useParams<{ name: string }>();
  const queryClient = useQueryClient();
  const [isEditing, setIsEditing] = useState(false);
  const [editedDefinition, setEditedDefinition] = useState('');

  const { data: pipeline, isLoading } = useQuery({
    queryKey: ['pipeline', name],
    queryFn: () => getPipeline(name!),
    enabled: !!name,
  });

  const { data: runs = [] } = useQuery({
    queryKey: ['pipeline-runs', name],
    queryFn: () => listPipelineRuns(name!),
    enabled: !!name,
    refetchInterval: 5000, // Refresh every 5 seconds
  });

  const updateMutation = useMutation({
    mutationFn: (definition: string) =>
      updatePipeline(name!, { definition }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipeline', name] });
      setIsEditing(false);
    },
  });

  const toggleMutation = useMutation({
    mutationFn: (enabled: boolean) =>
      updatePipeline(name!, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipeline', name] });
    },
  });

  const triggerMutation = useMutation({
    mutationFn: () => triggerPipelineRun(name!, { trigger_event: 'manual' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipeline-runs', name] });
    },
  });

  const handleEdit = () => {
    setEditedDefinition(pipeline?.definition || '');
    setIsEditing(true);
  };

  const handleSave = () => {
    updateMutation.mutate(editedDefinition);
  };

  const handleCancel = () => {
    setIsEditing(false);
    setEditedDefinition('');
  };

  const handleToggle = () => {
    if (pipeline) {
      toggleMutation.mutate(!pipeline.enabled);
    }
  };

  if (isLoading) {
    return <div className="text-center py-12">Loading...</div>;
  }

  if (!pipeline) {
    return (
      <div className="text-center py-12">
        <p className="mb-4">Pipeline not found</p>
        <Link to="/pipelines">
          <Button variant="outline">Back to Pipelines</Button>
        </Link>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center gap-4">
        <Link to="/pipelines">
          <Button variant="ghost" size="sm">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back
          </Button>
        </Link>
        <h1 className="text-3xl font-bold flex-1">{pipeline.name}</h1>
        <div className="flex items-center gap-2">
          {pipeline.enabled ? (
            <Badge variant="success">Enabled</Badge>
          ) : (
            <Badge variant="secondary">Disabled</Badge>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={handleToggle}
            disabled={toggleMutation.isPending}
          >
            {pipeline.enabled ? 'Disable' : 'Enable'}
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={() => triggerMutation.mutate()}
            disabled={!pipeline.enabled || triggerMutation.isPending}
          >
            <Play className="mr-2 h-4 w-4" />
            Run Pipeline
          </Button>
        </div>
      </div>

      {/* Pipeline Definition */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Pipeline Definition</CardTitle>
            {!isEditing && (
              <Button variant="outline" size="sm" onClick={handleEdit}>
                <Edit2 className="mr-2 h-4 w-4" />
                Edit
              </Button>
            )}
            {isEditing && (
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCancel}
                  disabled={updateMutation.isPending}
                >
                  <X className="mr-2 h-4 w-4" />
                  Cancel
                </Button>
                <Button
                  variant="default"
                  size="sm"
                  onClick={handleSave}
                  disabled={updateMutation.isPending}
                >
                  <Save className="mr-2 h-4 w-4" />
                  {updateMutation.isPending ? 'Saving...' : 'Save'}
                </Button>
              </div>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {isEditing ? (
            <textarea
              className="w-full h-96 font-mono text-sm p-4 border rounded-md bg-muted"
              value={editedDefinition}
              onChange={(e) => setEditedDefinition(e.target.value)}
              spellCheck={false}
            />
          ) : (
            <pre className="w-full h-96 overflow-auto font-mono text-sm p-4 border rounded-md bg-muted">
              {pipeline.definition}
            </pre>
          )}
        </CardContent>
      </Card>

      {/* Run History */}
      <Card>
        <CardHeader>
          <CardTitle>Run History</CardTitle>
        </CardHeader>
        <CardContent>
          {runs.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No runs yet
            </div>
          ) : (
            <div className="space-y-3">
              {runs.map((run) => (
                <Link
                  key={run.id}
                  to={`/pipelines/${encodeURIComponent(name!)}/runs/${run.id}`}
                  className="block"
                >
                  <div className="flex items-center justify-between p-4 border rounded-md hover:bg-muted transition-colors">
                    <div className="flex items-center gap-4">
                      <div className="font-mono text-sm">#{run.id}</div>
                      <PipelineRunStatusBadge status={run.status} />
                      {run.trigger_event && (
                        <Badge variant="secondary">{run.trigger_event}</Badge>
                      )}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {run.started_at
                        ? formatDate(run.started_at)
                        : formatDate(run.created_at)}
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
