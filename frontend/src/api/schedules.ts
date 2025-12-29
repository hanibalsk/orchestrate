import { apiRequest } from './client';
import type {
  Schedule,
  CreateScheduleRequest,
  UpdateScheduleRequest,
  ScheduleRun,
  Agent,
} from './types';

export async function listSchedules(): Promise<Schedule[]> {
  return apiRequest<Schedule[]>('/schedules');
}

export async function getSchedule(id: number): Promise<Schedule> {
  return apiRequest<Schedule>(`/schedules/${id}`);
}

export async function createSchedule(
  data: CreateScheduleRequest
): Promise<Schedule> {
  return apiRequest<Schedule>('/schedules', {
    method: 'POST',
    body: data,
  });
}

export async function updateSchedule(
  id: number,
  data: UpdateScheduleRequest
): Promise<Schedule> {
  return apiRequest<Schedule>(`/schedules/${id}`, {
    method: 'PUT',
    body: data,
  });
}

export async function deleteSchedule(id: number): Promise<void> {
  return apiRequest(`/schedules/${id}`, {
    method: 'DELETE',
  });
}

export async function pauseSchedule(id: number): Promise<Schedule> {
  return apiRequest<Schedule>(`/schedules/${id}/pause`, {
    method: 'POST',
  });
}

export async function resumeSchedule(id: number): Promise<Schedule> {
  return apiRequest<Schedule>(`/schedules/${id}/resume`, {
    method: 'POST',
  });
}

export async function runSchedule(id: number): Promise<Agent> {
  return apiRequest<Agent>(`/schedules/${id}/run`, {
    method: 'POST',
  });
}

export async function getScheduleRuns(id: number): Promise<ScheduleRun[]> {
  return apiRequest<ScheduleRun[]>(`/schedules/${id}/runs`);
}
