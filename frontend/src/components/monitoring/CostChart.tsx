import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { CostBreakdown } from '@/api/types';
import { DollarSign } from 'lucide-react';

interface CostChartProps {
  totalCost: number;
  breakdown: CostBreakdown[];
}

export function CostChart({ totalCost, breakdown }: CostChartProps) {
  // Handle undefined/null values with defaults
  const safeTotalCost = totalCost ?? 0;
  const safeBreakdown = breakdown ?? [];

  // Calculate percentages
  const breakdownWithPercentage = safeBreakdown.map((item) => ({
    ...item,
    percentage: safeTotalCost > 0 ? (item.total_cost / safeTotalCost) * 100 : 0,
  }));

  // Colors for the bars
  const colors = [
    'bg-blue-500',
    'bg-green-500',
    'bg-yellow-500',
    'bg-purple-500',
    'bg-pink-500',
    'bg-indigo-500',
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <DollarSign className="h-5 w-5" />
          Cost Breakdown
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Total Cost */}
          <div className="text-center">
            <div className="text-3xl font-bold">
              ${safeTotalCost.toFixed(2)}
            </div>
            <div className="text-sm text-muted-foreground">
              Total Cost (Current Period)
            </div>
          </div>

          {/* Breakdown by Agent Type */}
          {breakdownWithPercentage.length > 0 ? (
            <div className="space-y-4">
              <div className="text-sm font-medium">By Agent Type</div>
              {breakdownWithPercentage.map((item, index) => (
                <div key={item.agent_type} className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="font-medium capitalize">
                      {item.agent_type.replace(/_/g, ' ')}
                    </span>
                    <span className="text-muted-foreground">
                      ${item.total_cost.toFixed(2)} ({item.percentage.toFixed(1)}%)
                    </span>
                  </div>
                  {/* Bar chart */}
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className={`h-full ${colors[index % colors.length]} transition-all`}
                      style={{ width: `${item.percentage}%` }}
                    />
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {item.token_count.toLocaleString()} tokens
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8">
              <p className="text-sm text-muted-foreground">
                No cost data available for this period
              </p>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
