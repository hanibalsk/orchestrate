import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import type { Alert } from '@/api/types';
import { acknowledgeAlert } from '@/api/monitoring';
import { AlertCircle, CheckCircle, Info } from 'lucide-react';
import { cn } from '@/lib/utils';

interface AlertsListProps {
  alerts: Alert[];
}

// Backend uses snake_case, frontend uses PascalCase - handle both
const severityConfig: Record<
  string,
  { icon: React.ElementType; color: string; variant: 'default' | 'secondary' | 'destructive' }
> = {
  Info: {
    icon: Info,
    color: 'text-blue-500',
    variant: 'default',
  },
  info: {
    icon: Info,
    color: 'text-blue-500',
    variant: 'default',
  },
  Warning: {
    icon: AlertCircle,
    color: 'text-yellow-500',
    variant: 'secondary',
  },
  warning: {
    icon: AlertCircle,
    color: 'text-yellow-500',
    variant: 'secondary',
  },
  Critical: {
    icon: AlertCircle,
    color: 'text-red-500',
    variant: 'destructive',
  },
  critical: {
    icon: AlertCircle,
    color: 'text-red-500',
    variant: 'destructive',
  },
};

const defaultSeverityConfig = {
  icon: Info,
  color: 'text-gray-500',
  variant: 'default' as const,
};

export function AlertsList({ alerts }: AlertsListProps) {
  const queryClient = useQueryClient();
  const [acknowledgingId, setAcknowledgingId] = useState<number | null>(null);

  const acknowledgeMutation = useMutation({
    mutationFn: (alertId: number) =>
      acknowledgeAlert(alertId, {
        acknowledged_by: 'user',
        notes: 'Acknowledged from dashboard',
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alerts'] });
      queryClient.invalidateQueries({ queryKey: ['systemHealth'] });
      setAcknowledgingId(null);
    },
    onError: () => {
      setAcknowledgingId(null);
    },
  });

  const handleAcknowledge = (alertId: number) => {
    setAcknowledgingId(alertId);
    acknowledgeMutation.mutate(alertId);
  };

  if (alerts.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Active Alerts</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <CheckCircle className="h-12 w-12 text-green-500 mb-3" />
            <p className="text-sm text-muted-foreground">
              No active alerts. System is running smoothly.
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Active Alerts ({alerts.length})</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {alerts.map((alert) => {
            const config = severityConfig[alert.severity] || defaultSeverityConfig;
            const Icon = config.icon;
            const isAcknowledged = alert.status === 'Acknowledged';
            const isAcknowledging = acknowledgingId === alert.id;

            return (
              <div
                key={alert.id}
                className={cn(
                  'flex items-start gap-4 p-4 rounded-lg border',
                  isAcknowledged && 'bg-muted/50 opacity-75'
                )}
              >
                <Icon className={cn('h-5 w-5 mt-0.5', config.color)} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-start gap-2 mb-1">
                    <Badge variant={config.variant}>{alert.severity}</Badge>
                    {isAcknowledged && (
                      <Badge variant="outline">Acknowledged</Badge>
                    )}
                  </div>
                  <p className="text-sm font-medium mb-1">{alert.message}</p>
                  <p className="text-xs text-muted-foreground">
                    Triggered:{' '}
                    {new Date(alert.triggered_at).toLocaleString()}
                  </p>
                  {isAcknowledged && alert.acknowledged_by && (
                    <p className="text-xs text-muted-foreground">
                      Acknowledged by: {alert.acknowledged_by}
                    </p>
                  )}
                </div>
                {!isAcknowledged && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => handleAcknowledge(alert.id)}
                    disabled={isAcknowledging}
                  >
                    {isAcknowledging ? 'Acknowledging...' : 'Acknowledge'}
                  </Button>
                )}
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
