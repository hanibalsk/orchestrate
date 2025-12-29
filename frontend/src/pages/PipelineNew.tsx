import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useMutation } from '@tanstack/react-query';
import { ArrowLeft, Save } from 'lucide-react';
import { createPipeline } from '@/api/pipelines';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

const EXAMPLE_PIPELINE = `name: example-pipeline
description: Example deployment pipeline
version: 1

triggers:
  - event: pull_request.merged
    branches: [main]

stages:
  - name: test
    agent: tester
    task: "Run test suite"
    timeout: 30m
    on_failure: halt

  - name: deploy
    agent: deployer
    task: "Deploy to production"
    depends_on: test
    requires_approval: true
    approvers: [team-lead]
`;

export function PipelineNew() {
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [definition, setDefinition] = useState(EXAMPLE_PIPELINE);

  const createMutation = useMutation({
    mutationFn: () => createPipeline({ name, definition, enabled: true }),
    onSuccess: (data) => {
      navigate(`/pipelines/${encodeURIComponent(data.name)}`);
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      alert('Please enter a pipeline name');
      return;
    }
    if (!definition.trim()) {
      alert('Please enter a pipeline definition');
      return;
    }
    createMutation.mutate();
  };

  return (
    <div className="space-y-8">
      <div className="flex items-center gap-4">
        <Link to="/pipelines">
          <Button variant="ghost" size="sm">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back
          </Button>
        </Link>
        <h1 className="text-3xl font-bold">Create Pipeline</h1>
      </div>

      <form onSubmit={handleSubmit}>
        <div className="space-y-6">
          {/* Pipeline Name */}
          <Card>
            <CardHeader>
              <CardTitle>Pipeline Name</CardTitle>
            </CardHeader>
            <CardContent>
              <Input
                type="text"
                placeholder="my-pipeline"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={createMutation.isPending}
                required
              />
              <p className="mt-2 text-sm text-muted-foreground">
                Choose a unique name for your pipeline (lowercase, hyphens allowed)
              </p>
            </CardContent>
          </Card>

          {/* Pipeline Definition */}
          <Card>
            <CardHeader>
              <CardTitle>Pipeline Definition (YAML)</CardTitle>
            </CardHeader>
            <CardContent>
              <textarea
                className="w-full h-96 font-mono text-sm p-4 border rounded-md bg-muted"
                value={definition}
                onChange={(e) => setDefinition(e.target.value)}
                disabled={createMutation.isPending}
                spellCheck={false}
                required
              />
              <p className="mt-2 text-sm text-muted-foreground">
                Define your pipeline stages, dependencies, and triggers in YAML format
              </p>
            </CardContent>
          </Card>

          {/* Actions */}
          <div className="flex justify-end gap-2">
            <Link to="/pipelines">
              <Button
                type="button"
                variant="outline"
                disabled={createMutation.isPending}
              >
                Cancel
              </Button>
            </Link>
            <Button type="submit" disabled={createMutation.isPending}>
              <Save className="mr-2 h-4 w-4" />
              {createMutation.isPending ? 'Creating...' : 'Create Pipeline'}
            </Button>
          </div>

          {createMutation.isError && (
            <Card className="border-red-600 bg-red-50 dark:bg-red-950">
              <CardContent className="pt-6">
                <p className="text-sm text-red-600 dark:text-red-400">
                  Error creating pipeline:{' '}
                  {createMutation.error instanceof Error
                    ? createMutation.error.message
                    : 'Unknown error'}
                </p>
              </CardContent>
            </Card>
          )}
        </div>
      </form>
    </div>
  );
}
