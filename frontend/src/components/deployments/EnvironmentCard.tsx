import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge, DeploymentStatusBadge } from '@/components/ui/badge';
import { DeployButton } from './DeployButton';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { formatDistanceToNow } from '@/lib/time';
import type { Environment, Deployment } from '@/api/types';
import { Server, Undo2, ExternalLink } from 'lucide-react';

interface EnvironmentCardProps {
  environment: Environment;
  currentDeployment?: Deployment;
  onDeploy: (version: string, strategy?: string) => void;
  onRollback: () => void;
  isDeploying?: boolean;
}

export function EnvironmentCard({
  environment,
  currentDeployment,
  onDeploy,
  onRollback,
  isDeploying,
}: EnvironmentCardProps) {
  const [rollbackDialogOpen, setRollbackDialogOpen] = useState(false);

  const handleRollback = () => {
    onRollback();
    setRollbackDialogOpen(false);
  };

  const getEnvironmentVariant = (type: string) => {
    switch (type.toLowerCase()) {
      case 'production':
        return 'destructive';
      case 'staging':
        return 'warning';
      case 'development':
      case 'dev':
        return 'secondary';
      default:
        return 'default';
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Server className="h-5 w-5 text-muted-foreground" />
              <CardTitle>{environment.name}</CardTitle>
              <Badge variant={getEnvironmentVariant(environment.type)}>
                {environment.type}
              </Badge>
            </div>
            <div className="flex items-center gap-2">
              <DeployButton
                environment={environment.name}
                onDeploy={onDeploy}
                disabled={isDeploying}
              />
              {currentDeployment && currentDeployment.status.toLowerCase() === 'completed' && (
                <Button
                  variant="outline"
                  size="default"
                  onClick={() => setRollbackDialogOpen(true)}
                  disabled={isDeploying}
                >
                  <Undo2 className="mr-2 h-4 w-4" />
                  Rollback
                </Button>
              )}
            </div>
          </div>
        </CardHeader>

        <CardContent>
          <div className="space-y-3">
            {environment.url && (
              <div className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">URL:</span>
                <a
                  href={environment.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline flex items-center gap-1"
                >
                  {environment.url}
                  <ExternalLink className="h-3 w-3" />
                </a>
              </div>
            )}

            {currentDeployment ? (
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm text-muted-foreground">Current Version:</span>
                  <span className="font-semibold">v{currentDeployment.version}</span>
                  <DeploymentStatusBadge status={currentDeployment.status} />
                </div>

                <div className="text-sm text-muted-foreground">
                  Deployed {formatDistanceToNow(currentDeployment.started_at)}
                </div>

                {currentDeployment.provider && (
                  <div className="text-sm text-muted-foreground">
                    Provider: {currentDeployment.provider}
                    {currentDeployment.strategy && ` • ${currentDeployment.strategy}`}
                  </div>
                )}

                {currentDeployment.error_message && (
                  <div className="text-sm text-destructive mt-2">
                    Error: {currentDeployment.error_message}
                  </div>
                )}
              </div>
            ) : (
              <div className="text-sm text-muted-foreground">
                No deployments yet
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      <Dialog open={rollbackDialogOpen} onOpenChange={setRollbackDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rollback Deployment</DialogTitle>
            <DialogDescription>
              Are you sure you want to rollback the deployment in {environment.name}? This will
              revert to the previous version.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRollbackDialogOpen(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleRollback}>
              Rollback
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
