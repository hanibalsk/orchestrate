import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CheckCircle, Loader2, XCircle } from 'lucide-react';

interface DeploymentProgressProps {
  status: string;
  currentStep?: string;
  progress?: number;
}

export function DeploymentProgress({ status, currentStep, progress }: DeploymentProgressProps) {
  const getStatusIcon = () => {
    switch (status.toLowerCase()) {
      case 'completed':
        return <CheckCircle className="h-5 w-5 text-success" />;
      case 'failed':
        return <XCircle className="h-5 w-5 text-destructive" />;
      case 'inprogress':
      case 'in_progress':
        return <Loader2 className="h-5 w-5 text-primary animate-spin" />;
      default:
        return <Loader2 className="h-5 w-5 text-muted-foreground" />;
    }
  };

  const getStatusText = () => {
    switch (status.toLowerCase()) {
      case 'completed':
        return 'Deployment Completed';
      case 'failed':
        return 'Deployment Failed';
      case 'inprogress':
      case 'in_progress':
        return 'Deployment In Progress';
      case 'pending':
        return 'Deployment Pending';
      default:
        return status;
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3">
          {getStatusIcon()}
          <CardTitle className="text-lg">{getStatusText()}</CardTitle>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {currentStep && (
          <div className="text-sm text-muted-foreground">
            Current Step: <span className="font-medium">{currentStep}</span>
          </div>
        )}

        {progress !== undefined && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Progress</span>
              <span className="font-medium">{Math.round(progress)}%</span>
            </div>
            {/* Note: Progress component would need to be created */}
            <div className="w-full bg-secondary h-2 rounded-full overflow-hidden">
              <div
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
