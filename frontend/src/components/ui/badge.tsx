import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';
import type { AgentState, AgentType } from '@/api/types';

const badgeVariants = cva(
  'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
  {
    variants: {
      variant: {
        default:
          'border-transparent bg-primary text-primary-foreground shadow',
        secondary:
          'border-transparent bg-secondary text-secondary-foreground',
        destructive:
          'border-transparent bg-destructive text-destructive-foreground shadow',
        outline: 'text-foreground',
        success: 'border-transparent bg-success text-white',
        // Agent states
        created: 'border-transparent bg-gray-500 text-white',
        initializing: 'border-transparent bg-cyan-600 text-white',
        running: 'border-transparent bg-success text-white',
        waiting_for_input: 'border-transparent bg-warning text-black',
        waiting_for_external: 'border-transparent bg-orange-500 text-white',
        paused: 'border-transparent bg-warning text-black',
        completed: 'border-transparent bg-info text-white',
        failed: 'border-transparent bg-danger text-white',
        terminated: 'border-transparent bg-gray-600 text-white',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

// Helper component for agent state badges
export function AgentStateBadge({ state }: { state: AgentState }) {
  const labels: Record<AgentState, string> = {
    created: 'Created',
    initializing: 'Initializing',
    running: 'Running',
    waiting_for_input: 'Waiting for Input',
    waiting_for_external: 'Waiting',
    paused: 'Paused',
    completed: 'Completed',
    failed: 'Failed',
    terminated: 'Terminated',
  };

  return <Badge variant={state}>{labels[state]}</Badge>;
}

// Helper component for agent type badges
export function AgentTypeBadge({ type }: { type: AgentType }) {
  const labels: Record<AgentType, string> = {
    // Development agents
    story_developer: 'Story Developer',
    code_reviewer: 'Code Reviewer',
    issue_fixer: 'Issue Fixer',
    explorer: 'Explorer',
    // BMAD agents
    bmad_orchestrator: 'BMAD Orchestrator',
    bmad_planner: 'BMAD Planner',
    // PR management
    pr_shepherd: 'PR Shepherd',
    pr_controller: 'PR Controller',
    conflict_resolver: 'Conflict Resolver',
    // System agents
    background_controller: 'Background Controller',
    scheduler: 'Scheduler',
  };

  return <Badge variant="secondary">{labels[type]}</Badge>;
}

export { Badge, badgeVariants };
