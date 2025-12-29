import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ModuleCoverage } from '@/api/test-types';

interface ModuleCoverageTableProps {
  modules?: ModuleCoverage[];
  isLoading?: boolean;
}

export function ModuleCoverageTable({
  modules,
  isLoading,
}: ModuleCoverageTableProps) {
  const [expandedModules, setExpandedModules] = useState<Set<string>>(
    new Set()
  );

  const toggleModule = (moduleName: string) => {
    setExpandedModules((prev) => {
      const next = new Set(prev);
      if (next.has(moduleName)) {
        next.delete(moduleName);
      } else {
        next.add(moduleName);
      }
      return next;
    });
  };

  const getCoverageColor = (percent: number) => {
    if (percent >= 80) return 'text-success';
    if (percent >= 50) return 'text-warning';
    return 'text-destructive';
  };

  const getProgressBarColor = (percent: number) => {
    if (percent >= 80) return 'bg-success';
    if (percent >= 50) return 'bg-warning';
    return 'bg-destructive';
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Module Coverage</CardTitle>
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
          <CardTitle>Module Coverage</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center text-muted-foreground">
            No module coverage data available
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Module Coverage</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {modules.map((module) => {
            const isExpanded = expandedModules.has(module.module_name);
            return (
              <Collapsible
                key={module.module_name}
                open={isExpanded}
                onOpenChange={() => toggleModule(module.module_name)}
              >
                <CollapsibleTrigger className="w-full">
                  <div className="flex items-center gap-2 p-3 rounded hover:bg-accent">
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                    <div className="flex-1 flex items-center justify-between">
                      <div className="text-left">
                        <div className="font-medium">{module.module_name}</div>
                        <div className="text-xs text-muted-foreground">
                          {module.files.length} file
                          {module.files.length !== 1 ? 's' : ''}
                        </div>
                      </div>
                      <div className="flex items-center gap-4">
                        <div className="text-right">
                          <div
                            className={cn(
                              'font-medium',
                              getCoverageColor(module.coverage_percent)
                            )}
                          >
                            {module.coverage_percent.toFixed(1)}%
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {module.lines_covered}/{module.lines_total} lines
                          </div>
                        </div>
                        <div className="w-32 h-2 bg-secondary rounded-full overflow-hidden">
                          <div
                            className={cn(
                              'h-full transition-all',
                              getProgressBarColor(module.coverage_percent)
                            )}
                            style={{ width: `${module.coverage_percent}%` }}
                          />
                        </div>
                      </div>
                    </div>
                  </div>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="ml-6 mt-2 space-y-1">
                    {module.files.map((file) => (
                      <div
                        key={file.file_path}
                        className="flex items-center justify-between p-2 rounded text-sm hover:bg-accent"
                      >
                        <div className="flex-1 truncate" title={file.file_path}>
                          {file.file_path}
                        </div>
                        <div className="flex items-center gap-4 ml-4">
                          <div
                            className={cn(
                              'font-medium',
                              getCoverageColor(file.coverage_percent)
                            )}
                          >
                            {file.coverage_percent.toFixed(1)}%
                          </div>
                          <div className="text-xs text-muted-foreground w-24 text-right">
                            {file.lines_covered}/{file.lines_total} lines
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
