import { apiRequest } from './client';
import type {
  Environment,
  Deployment,
  CreateDeploymentRequest,
  Release,
  CreateReleaseRequest,
} from './types';

// Environment APIs
export async function listEnvironments(): Promise<Environment[]> {
  return apiRequest('/environments');
}

export async function getEnvironment(name: string): Promise<Environment> {
  return apiRequest(`/environments/${encodeURIComponent(name)}`);
}

// Deployment APIs
export async function listDeployments(
  environment?: string,
  limit?: number
): Promise<Deployment[]> {
  const params = new URLSearchParams();
  if (environment) params.append('environment', environment);
  if (limit) params.append('limit', limit.toString());

  const query = params.toString();
  return apiRequest(`/deployments${query ? `?${query}` : ''}`);
}

export async function getDeployment(id: number): Promise<Deployment> {
  return apiRequest(`/deployments/${id}`);
}

export async function createDeployment(
  request: CreateDeploymentRequest
): Promise<Deployment> {
  return apiRequest('/deployments', {
    method: 'POST',
    body: request,
  });
}

export async function rollbackDeployment(id: number): Promise<Deployment> {
  return apiRequest(`/deployments/${id}/rollback`, {
    method: 'POST',
  });
}

// Release APIs
export async function listReleases(): Promise<Release[]> {
  return apiRequest('/releases');
}

export async function createRelease(
  request: CreateReleaseRequest
): Promise<Release> {
  return apiRequest('/releases', {
    method: 'POST',
    body: request,
  });
}

export async function publishRelease(version: string): Promise<Release> {
  return apiRequest(`/releases/${encodeURIComponent(version)}/publish`, {
    method: 'POST',
  });
}
