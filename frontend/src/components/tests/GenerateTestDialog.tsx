import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { FileCode, Loader2 } from 'lucide-react';
import { generateTests } from '@/api/tests';
import type { TestType, Language } from '@/api/test-types';

interface GenerateTestDialogProps {
  filePath?: string;
}

export function GenerateTestDialog({ filePath }: GenerateTestDialogProps) {
  const [open, setOpen] = useState(false);
  const [testType, setTestType] = useState<TestType>('unit');
  const [language, setLanguage] = useState<Language>('rust');
  const [target, setTarget] = useState(filePath || '');
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: generateTests,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['test-runs'] });
      setOpen(false);
    },
  });

  const handleGenerate = () => {
    if (!target && testType !== 'e2e') {
      return;
    }

    mutation.mutate({
      test_type: testType,
      target: testType !== 'e2e' ? target : undefined,
      language,
    });
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <FileCode className="h-4 w-4 mr-2" />
          Generate Tests
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Generate Tests</DialogTitle>
          <DialogDescription>
            Automatically generate test cases for your code
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Test Type */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Test Type</label>
            <Select
              value={testType}
              onValueChange={(value) => setTestType(value as TestType)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="unit">Unit Tests</SelectItem>
                <SelectItem value="integration">Integration Tests</SelectItem>
                <SelectItem value="e2e">E2E Tests</SelectItem>
                <SelectItem value="property">Property Tests</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Language */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Language</label>
            <Select
              value={language}
              onValueChange={(value) => setLanguage(value as Language)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="rust">Rust</SelectItem>
                <SelectItem value="typescript">TypeScript</SelectItem>
                <SelectItem value="python">Python</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Target File */}
          {testType !== 'e2e' && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Target File/Module</label>
              <input
                type="text"
                value={target}
                onChange={(e) => setTarget(e.target.value)}
                placeholder="e.g., src/example.rs"
                className="w-full px-3 py-2 border rounded-md"
              />
            </div>
          )}

          {mutation.error && (
            <div className="text-sm text-destructive">
              Error: {mutation.error.message}
            </div>
          )}

          {mutation.data && (
            <div className="text-sm text-success">
              Generated {mutation.data.generated_count} test case
              {mutation.data.generated_count !== 1 ? 's' : ''}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleGenerate}
            disabled={mutation.isPending || (!target && testType !== 'e2e')}
          >
            {mutation.isPending && (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            )}
            Generate
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
