import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import type { PipelineRunStatus, PipelineStageStatus } from '@/api/types';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString();
}

export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

// Pipeline status utilities
export function getPipelineRunStatusColor(
  status: PipelineRunStatus
): 'default' | 'success' | 'warning' | 'destructive' | 'secondary' {
  switch (status) {
    case 'Succeeded':
      return 'success';
    case 'Running':
      return 'default';
    case 'WaitingApproval':
      return 'warning';
    case 'Failed':
      return 'destructive';
    case 'Cancelled':
      return 'secondary';
    case 'Pending':
    default:
      return 'secondary';
  }
}

export function getPipelineStageStatusColor(
  status: PipelineStageStatus
): 'default' | 'success' | 'warning' | 'destructive' | 'secondary' {
  switch (status) {
    case 'Succeeded':
      return 'success';
    case 'Running':
      return 'default';
    case 'WaitingApproval':
      return 'warning';
    case 'Failed':
      return 'destructive';
    case 'Cancelled':
    case 'Skipped':
      return 'secondary';
    case 'Pending':
    default:
      return 'secondary';
  }
}

export function formatDuration(startedAt: string | null, completedAt: string | null): string {
  if (!startedAt) return '-';

  const start = new Date(startedAt);
  const end = completedAt ? new Date(completedAt) : new Date();
  const diffMs = end.getTime() - start.getTime();

  const seconds = Math.floor(diffMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}
