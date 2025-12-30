import { Card, CardContent } from '@/components/ui/card';
import { TrendingUp, TrendingDown } from 'lucide-react';
import { cn } from '@/lib/utils';

interface MetricCardProps {
  label: string;
  value: string | number;
  icon?: React.ReactNode;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
  variant?: 'default' | 'success' | 'warning' | 'danger';
}

export function MetricCard({
  label,
  value,
  icon,
  trend,
  trendValue,
  variant = 'default',
}: MetricCardProps) {
  const variantColors = {
    default: 'text-foreground',
    success: 'text-green-600',
    warning: 'text-yellow-600',
    danger: 'text-red-600',
  };

  const TrendIcon = trend === 'up' ? TrendingUp : trend === 'down' ? TrendingDown : null;

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <p className="text-sm font-medium text-muted-foreground">{label}</p>
            <div className="mt-2 flex items-baseline gap-2">
              <p className={cn('text-2xl font-bold', variantColors[variant])}>
                {value}
              </p>
              {trend && TrendIcon && trendValue && (
                <div className="flex items-center gap-1 text-xs">
                  <TrendIcon
                    className={cn(
                      'h-3 w-3',
                      trend === 'up' ? 'text-green-600' : 'text-red-600'
                    )}
                  />
                  <span
                    className={cn(
                      trend === 'up' ? 'text-green-600' : 'text-red-600'
                    )}
                  >
                    {trendValue}
                  </span>
                </div>
              )}
            </div>
          </div>
          {icon && (
            <div className="rounded-lg bg-muted p-2">
              {icon}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
