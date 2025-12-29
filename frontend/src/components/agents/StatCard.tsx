import { Card, CardContent } from '@/components/ui/card';
import { cn } from '@/lib/utils';

interface StatCardProps {
  label: string;
  value: number;
  variant?: 'default' | 'success' | 'warning' | 'info';
}

export function StatCard({ label, value, variant = 'default' }: StatCardProps) {
  const valueClasses = {
    default: 'text-foreground',
    success: 'text-success',
    warning: 'text-warning',
    info: 'text-info',
  };

  return (
    <Card>
      <CardContent className="p-6 text-center">
        <div className={cn('text-4xl font-bold', valueClasses[variant])}>
          {value}
        </div>
        <div className="text-sm text-muted-foreground uppercase mt-1">
          {label}
        </div>
      </CardContent>
    </Card>
  );
}
