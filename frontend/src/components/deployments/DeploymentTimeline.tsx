import { formatDistanceToNow } from '@/lib/time';
import { DeploymentStatusBadge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import type { Deployment } from '@/api/types';
import { CheckCircle, XCircle, Clock, ArrowRight } from 'lucide-react';

interface DeploymentTimelineProps {
  deployments: Deployment[];
}

export function DeploymentTimeline({ deployments }: DeploymentTimelineProps) {
  if (deployments.length === 0) {
    return (
      <Card>
        <CardContent className="py-12">
          <div className="text-center text-muted-foreground">
            No deployment history yet
          </div>
        </CardContent>
      </Card>
    );
  }

  const getIcon = (status: string) => {
    switch (status.toLowerCase()) {
      case 'completed':
        return <CheckCircle className="h-5 w-5 text-success" />;
      case 'failed':
        return <XCircle className="h-5 w-5 text-destructive" />;
      case 'inprogress':
      case 'in_progress':
        return <Clock className="h-5 w-5 text-primary animate-pulse" />;
      default:
        return <Clock className="h-5 w-5 text-muted-foreground" />;
    }
  };

  const getDuration = (deployment: Deployment) => {
    if (!deployment.completed_at) {
      return 'In progress...';
    }
    const start = new Date(deployment.started_at);
    const end = new Date(deployment.completed_at);
    const durationMs = end.getTime() - start.getTime();
    const durationSec = Math.floor(durationMs / 1000);

    if (durationSec < 60) {
      return `${durationSec}s`;
    } else if (durationSec < 3600) {
      const minutes = Math.floor(durationSec / 60);
      const seconds = durationSec % 60;
      return `${minutes}m ${seconds}s`;
    } else {
      const hours = Math.floor(durationSec / 3600);
      const minutes = Math.floor((durationSec % 3600) / 60);
      return `${hours}h ${minutes}m`;
    }
  };

  return (
    <div className="space-y-4">
      {deployments.map((deployment, index) => (
        <div key={deployment.id} className="flex gap-4">
          <div className="flex flex-col items-center">
            <div className="rounded-full bg-background border-2 border-border p-2">
              {getIcon(deployment.status)}
            </div>
            {index < deployments.length - 1 && (
              <div className="flex-1 w-0.5 bg-border my-2" style={{ minHeight: '2rem' }} />
            )}
          </div>

          <Card className="flex-1">
            <CardContent className="py-4">
              <div className="flex items-start justify-between">
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold">v{deployment.version}</span>
                    <ArrowRight className="h-4 w-4 text-muted-foreground" />
                    <span className="text-muted-foreground">{deployment.environment_name}</span>
                    <DeploymentStatusBadge status={deployment.status} />
                  </div>

                  <div className="flex items-center gap-4 text-sm text-muted-foreground">
                    <span>{formatDistanceToNow(deployment.started_at)}</span>
                    <span>•</span>
                    <span>{getDuration(deployment)}</span>
                    <span>•</span>
                    <span>{deployment.provider}</span>
                    {deployment.strategy && (
                      <>
                        <span>•</span>
                        <span>{deployment.strategy}</span>
                      </>
                    )}
                  </div>

                  {deployment.error_message && (
                    <div className="mt-2 text-sm text-destructive">
                      Error: {deployment.error_message}
                    </div>
                  )}
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      ))}
    </div>
  );
}
