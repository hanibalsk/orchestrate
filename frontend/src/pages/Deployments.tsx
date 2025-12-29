import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { EnvironmentCard } from '@/components/deployments/EnvironmentCard';
import { DeploymentTimeline } from '@/components/deployments/DeploymentTimeline';
import {
  listEnvironments,
  listDeployments,
  createDeployment,
  rollbackDeployment,
} from '@/api/deployments';
import type { Deployment } from '@/api/types';
import { History, GitCompare } from 'lucide-react';

export function Deployments() {
  const queryClient = useQueryClient();
  const [selectedEnvironment, setSelectedEnvironment] = useState<string | null>(null);
  const [showComparison, setShowComparison] = useState(false);

  const { data: environments = [], isLoading: isLoadingEnvs } = useQuery({
    queryKey: ['environments'],
    queryFn: listEnvironments,
  });

  const { data: allDeployments = [], isLoading: isLoadingDeployments } = useQuery({
    queryKey: ['deployments'],
    queryFn: () => listDeployments(undefined, 50),
  });

  const deployMutation = useMutation({
    mutationFn: (params: { environment: string; version: string; strategy?: string }) =>
      createDeployment({
        environment: params.environment,
        version: params.version,
        strategy: params.strategy,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deployments'] });
    },
  });

  const rollbackMutation = useMutation({
    mutationFn: (deploymentId: number) => rollbackDeployment(deploymentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deployments'] });
    },
  });

  const getCurrentDeployment = (envName: string): Deployment | undefined => {
    return allDeployments
      .filter((d) => d.environment_name === envName)
      .sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime())[0];
  };

  const getEnvironmentDeployments = (envName: string): Deployment[] => {
    return allDeployments
      .filter((d) => d.environment_name === envName)
      .sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime())
      .slice(0, 10);
  };

  const handleDeploy = (environment: string) => (version: string, strategy?: string) => {
    deployMutation.mutate({ environment, version, strategy });
  };

  const handleRollback = (deployment: Deployment) => () => {
    rollbackMutation.mutate(deployment.id);
  };

  const getVersionComparison = () => {
    const versions = new Map<string, Set<string>>();
    environments.forEach((env) => {
      const current = getCurrentDeployment(env.name);
      if (current) {
        if (!versions.has(current.version)) {
          versions.set(current.version, new Set());
        }
        versions.get(current.version)!.add(env.name);
      }
    });
    return versions;
  };

  if (isLoadingEnvs || isLoadingDeployments) {
    return (
      <div className="space-y-8">
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold">Deployments</h1>
        </div>
        <div className="text-center py-12 text-muted-foreground">Loading...</div>
      </div>
    );
  }

  const versionComparison = getVersionComparison();

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Deployments</h1>
        <div className="flex gap-2">
          <Button
            variant={showComparison ? 'default' : 'outline'}
            onClick={() => setShowComparison(!showComparison)}
          >
            <GitCompare className="mr-2 h-4 w-4" />
            Compare Environments
          </Button>
        </div>
      </div>

      {showComparison && (
        <Card>
          <CardHeader>
            <CardTitle>Version Comparison</CardTitle>
          </CardHeader>
          <CardContent>
            {versionComparison.size === 0 ? (
              <div className="text-muted-foreground">No deployments to compare</div>
            ) : (
              <div className="space-y-3">
                {Array.from(versionComparison.entries()).map(([version, envs]) => (
                  <div key={version} className="flex items-center gap-4">
                    <span className="font-semibold min-w-24">v{version}</span>
                    <div className="flex gap-2 flex-wrap">
                      {Array.from(envs).map((env) => {
                        const environment = environments.find((e) => e.name === env);
                        const variant =
                          environment?.type.toLowerCase() === 'production'
                            ? 'destructive'
                            : environment?.type.toLowerCase() === 'staging'
                            ? 'warning'
                            : 'secondary';
                        return (
                          <span
                            key={env}
                            className={`px-3 py-1 rounded-full text-xs font-medium ${
                              variant === 'destructive'
                                ? 'bg-destructive text-destructive-foreground'
                                : variant === 'warning'
                                ? 'bg-yellow-600 text-white'
                                : 'bg-secondary text-secondary-foreground'
                            }`}
                          >
                            {env}
                          </span>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {environments.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center text-muted-foreground">
              No environments configured yet
            </div>
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="space-y-4">
            <h2 className="text-xl font-semibold">Environments</h2>
            <div className="grid gap-4 md:grid-cols-2">
              {environments.map((env) => (
                <EnvironmentCard
                  key={env.id}
                  environment={env}
                  currentDeployment={getCurrentDeployment(env.name)}
                  onDeploy={handleDeploy(env.name)}
                  onRollback={handleRollback(getCurrentDeployment(env.name)!)}
                  isDeploying={deployMutation.isPending || rollbackMutation.isPending}
                />
              ))}
            </div>
          </div>

          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <History className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Deployment History</h2>
            </div>

            {environments.length > 1 && (
              <div className="flex gap-2 flex-wrap">
                <Button
                  variant={selectedEnvironment === null ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setSelectedEnvironment(null)}
                >
                  All Environments
                </Button>
                {environments.map((env) => (
                  <Button
                    key={env.id}
                    variant={selectedEnvironment === env.name ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => setSelectedEnvironment(env.name)}
                  >
                    {env.name}
                  </Button>
                ))}
              </div>
            )}

            <DeploymentTimeline
              deployments={
                selectedEnvironment
                  ? getEnvironmentDeployments(selectedEnvironment)
                  : allDeployments.slice(0, 20)
              }
            />
          </div>
        </>
      )}
    </div>
  );
}
