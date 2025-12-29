import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ModuleCoverage, FileCoverage } from '@/api/test-types';

interface UntestedCodeListProps {
  modules?: ModuleCoverage[];
  isLoading?: boolean;
  threshold?: number;
}

export function UntestedCodeList({
  modules,
  isLoading,
  threshold = 50,
}: UntestedCodeListProps) {
  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-warning" />
            Untested Code
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  if (!modules || modules.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-warning" />
            Untested Code
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">
            No coverage data available
          </div>
        </CardContent>
      </Card>
    );
  }

  // Collect all files below threshold
  const untertestedFiles: Array<FileCoverage & { moduleName: string }> = [];
  modules.forEach((module) => {
    module.files.forEach((file) => {
      if (file.coverage_percent < threshold) {
        untertestedFiles.push({
          ...file,
          moduleName: module.module_name,
        });
      }
    });
  });

  // Sort by coverage percent (lowest first)
  untertestedFiles.sort((a, b) => a.coverage_percent - b.coverage_percent);

  const getSeverityBadge = (percent: number) => {
    if (percent < 20) {
      return (
        <Badge variant="destructive" className="text-xs">
          Critical
        </Badge>
      );
    }
    if (percent < 40) {
      return (
        <Badge className="text-xs bg-orange-500">High</Badge>
      );
    }
    return (
      <Badge className="text-xs bg-yellow-500">Medium</Badge>
    );
  };

  if (untertestedFiles.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-success" />
            Untested Code
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-success">
            All files meet the {threshold}% coverage threshold!
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <AlertTriangle className="h-5 w-5 text-warning" />
          Untested Code
          <Badge variant="secondary">{untertestedFiles.length} files</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {untertestedFiles.map((file) => (
            <div
              key={`${file.moduleName}:${file.file_path}`}
              className="flex items-center justify-between p-3 rounded border hover:bg-accent"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  {getSeverityBadge(file.coverage_percent)}
                  <div className="text-sm font-medium truncate" title={file.file_path}>
                    {file.file_path}
                  </div>
                </div>
                <div className="text-xs text-muted-foreground mt-1">
                  {file.moduleName}
                </div>
              </div>
              <div className="ml-4 text-right">
                <div
                  className={cn(
                    'text-lg font-bold',
                    file.coverage_percent < 20 && 'text-destructive',
                    file.coverage_percent >= 20 &&
                      file.coverage_percent < 40 &&
                      'text-orange-500',
                    file.coverage_percent >= 40 && 'text-yellow-500'
                  )}
                >
                  {file.coverage_percent.toFixed(1)}%
                </div>
                <div className="text-xs text-muted-foreground">
                  {file.lines_covered}/{file.lines_total} lines
                </div>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
