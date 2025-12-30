import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { ComponentHealth, HealthStatus as HealthStatusType } from '@/api/types';
import { AlertCircle, CheckCircle, XCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

interface HealthStatusProps {
  status: HealthStatusType;
  components: ComponentHealth[];
}

export function HealthStatus({ status, components }: HealthStatusProps) {
  const statusConfig = {
    Healthy: {
      icon: CheckCircle,
      color: 'text-green-500',
      bgColor: 'bg-green-50',
      label: 'Healthy',
    },
    Degraded: {
      icon: AlertCircle,
      color: 'text-yellow-500',
      bgColor: 'bg-yellow-50',
      label: 'Degraded',
    },
    Unhealthy: {
      icon: XCircle,
      color: 'text-red-500',
      bgColor: 'bg-red-50',
      label: 'Unhealthy',
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <Card>
      <CardHeader>
        <CardTitle>System Health</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Overall Status */}
          <div className={cn('flex items-center gap-3 p-4 rounded-lg', config.bgColor)}>
            <Icon className={cn('h-6 w-6', config.color)} />
            <div>
              <div className="font-semibold">{config.label}</div>
              <div className="text-sm text-muted-foreground">
                Overall system status
              </div>
            </div>
          </div>

          {/* Component Status */}
          <div className="space-y-2">
            <div className="text-sm font-medium">Components</div>
            {components.map((component) => {
              const componentConfig = statusConfig[component.status];
              const ComponentIcon = componentConfig.icon;

              return (
                <div
                  key={component.name}
                  className="flex items-center justify-between p-3 rounded-lg border"
                >
                  <div className="flex items-center gap-2">
                    <ComponentIcon
                      className={cn('h-4 w-4', componentConfig.color)}
                    />
                    <span className="text-sm font-medium capitalize">
                      {component.name}
                    </span>
                  </div>
                  {component.message && (
                    <span className="text-xs text-muted-foreground">
                      {component.message}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
