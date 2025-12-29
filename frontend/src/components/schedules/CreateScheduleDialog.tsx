import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createSchedule } from '@/api/schedules';
import type { AgentType } from '@/api/types';
import { Button } from '@/components/ui/button';
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
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Plus } from 'lucide-react';

const agentTypes: { value: AgentType; label: string }[] = [
  { value: 'story_developer', label: 'Story Developer' },
  { value: 'code_reviewer', label: 'Code Reviewer' },
  { value: 'issue_fixer', label: 'Issue Fixer' },
  { value: 'explorer', label: 'Explorer' },
  { value: 'bmad_orchestrator', label: 'BMAD Orchestrator' },
  { value: 'bmad_planner', label: 'BMAD Planner' },
  { value: 'pr_shepherd', label: 'PR Shepherd' },
  { value: 'pr_controller', label: 'PR Controller' },
  { value: 'conflict_resolver', label: 'Conflict Resolver' },
  { value: 'background_controller', label: 'Background Controller' },
  { value: 'scheduler', label: 'Scheduler' },
];

const cronPresets = [
  { value: '@hourly', label: 'Every hour' },
  { value: '@daily', label: 'Daily at midnight' },
  { value: '@weekly', label: 'Weekly on Sunday' },
  { value: '0 2 * * *', label: 'Daily at 2 AM' },
  { value: '0 0 * * 0', label: 'Weekly on Sunday at midnight' },
  { value: '*/15 * * * *', label: 'Every 15 minutes' },
  { value: '0 9 * * 1-5', label: 'Weekdays at 9 AM' },
  { value: 'custom', label: 'Custom expression' },
];

export function CreateScheduleDialog() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [cronPreset, setCronPreset] = useState('@daily');
  const [cronExpression, setCronExpression] = useState('@daily');
  const [agentType, setAgentType] = useState<AgentType>('background_controller');
  const [task, setTask] = useState('');
  const [enabled, setEnabled] = useState(true);
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: createSchedule,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['schedules'] });
      setOpen(false);
      resetForm();
    },
  });

  const resetForm = () => {
    setName('');
    setCronPreset('@daily');
    setCronExpression('@daily');
    setAgentType('background_controller');
    setTask('');
    setEnabled(true);
  };

  const handleCronPresetChange = (value: string) => {
    setCronPreset(value);
    if (value !== 'custom') {
      setCronExpression(value);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !cronExpression.trim() || !task.trim()) return;

    mutation.mutate({
      name: name.trim(),
      cron_expression: cronExpression.trim(),
      agent_type: agentType,
      task: task.trim(),
      enabled,
    });
  };

  const isValid = name.trim() && cronExpression.trim() && task.trim();

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus className="h-4 w-4 mr-2" />
          Create Schedule
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-2xl">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create New Schedule</DialogTitle>
            <DialogDescription>
              Schedule an agent to run automatically at specified times.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="name">Schedule Name</Label>
              <Input
                id="name"
                placeholder="e.g., daily-security-scan"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="cron-preset">Schedule Frequency</Label>
              <Select value={cronPreset} onValueChange={handleCronPresetChange}>
                <SelectTrigger id="cron-preset">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {cronPresets.map((preset) => (
                    <SelectItem key={preset.value} value={preset.value}>
                      {preset.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {cronPreset === 'custom' && (
              <div className="space-y-2">
                <Label htmlFor="cron">Cron Expression</Label>
                <Input
                  id="cron"
                  placeholder="0 2 * * *"
                  value={cronExpression}
                  onChange={(e) => setCronExpression(e.target.value)}
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Format: minute hour day month weekday (e.g., "0 2 * * *" for
                  daily at 2 AM)
                </p>
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="agent-type">Agent Type</Label>
              <Select
                value={agentType}
                onValueChange={(value) => setAgentType(value as AgentType)}
              >
                <SelectTrigger id="agent-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {agentTypes.map((type) => (
                    <SelectItem key={type.value} value={type.value}>
                      {type.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="task">Task Description</Label>
              <Textarea
                id="task"
                className="min-h-[100px]"
                placeholder="Describe what the agent should do..."
                value={task}
                onChange={(e) => setTask(e.target.value)}
                required
              />
            </div>

            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="enabled"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
                className="h-4 w-4 rounded border-gray-300"
              />
              <Label htmlFor="enabled" className="cursor-pointer">
                Enable schedule immediately
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setOpen(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={mutation.isPending || !isValid}>
              {mutation.isPending ? 'Creating...' : 'Create Schedule'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
