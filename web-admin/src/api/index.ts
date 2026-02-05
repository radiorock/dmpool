// API client for DMPool Admin API

import type {
  PoolStats,
  MinerInfo,
  PaymentInfo,
  NotificationConfig,
  SystemConfig,
} from '../types';

const API_BASE_URL = import.meta.env.VITE_ADMIN_API_URL || 'http://localhost:8080/admin';

// Auth token for admin API
const getAuthToken = () => localStorage.getItem('admin_token') || '';

// Request wrapper with auth
async function request<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  };

  const token = getAuthToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  try {
    const response = await fetch(url, { ...options, headers });
    const data = await response.json();

    if (!response.ok) {
      throw new Error(data.error || `HTTP ${response.status}`);
    }

    return data as T;
  } catch (error) {
    console.error('API request failed:', error);
    throw error;
  }
}

// Dashboard APIs
export async function fetchDashboardStats(): Promise<PoolStats> {
  return request<PoolStats>('/dashboard/stats');
}

// Miner Management APIs
export async function fetchMiners(params: {
  search?: string;
  limit?: number;
  offset?: number;
}): Promise<{ miners: MinerInfo[]; total: number }> {
  const query = new URLSearchParams(params as Record<string, string>).toString();
  return request<{ miners: MinerInfo[]; total: number }>(`/miners?${query}`);
}

export async function banMiner(address: string): Promise<void> {
  return request<void>(`/miners/${address}/ban`, { method: 'POST' });
}

export async function unbanMiner(address: string): Promise<void> {
  return request<void>(`/miners/${address}/unban`, { method: 'POST' });
}

export async function updateMinerThreshold(
  address: string,
  threshold: number
): Promise<void> {
  return request<void>(`/miners/${address}/threshold`, {
    method: 'PATCH',
    body: JSON.stringify({ threshold }),
  });
}

// Payment Management APIs
export async function fetchPayments(params: {
  status?: string;
  limit?: number;
  offset?: number;
}): Promise<{ payments: PaymentInfo[]; total: number }> {
  const query = new URLSearchParams(params as Record<string, string>).toString();
  return request<{ payments: PaymentInfo[]; total: number }>(`/payments?${query}`);
}

export async function triggerPayment(): Promise<void> {
  return request<void>('/payments/trigger', { method: 'POST' });
}

// Notification APIs
export async function fetchNotifications(): Promise<NotificationConfig[]> {
  return request<NotificationConfig[]>('/notifications');
}

export async function updateNotification(
  id: number,
  config: Partial<NotificationConfig>
): Promise<void> {
  return request<void>(`/notifications/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(config),
  });
}

// System Config APIs
export async function fetchConfigs(): Promise<SystemConfig[]> {
  return request<SystemConfig[]>('/config');
}

export async function updateConfig(
  key: string,
  value: string
): Promise<void> {
  return request<void>(`/config/${key}`, {
    method: 'PATCH',
    body: JSON.stringify({ value }),
  });
}

// Utility functions
export function formatHashrate(hashesPerSecond: number): string {
  if (hashesPerSecond < 1000) {
    return `${hashesPerSecond.toFixed(2)} H/s`;
  }
  if (hashesPerSecond < 1_000_000) {
    return `${(hashesPerSecond / 1000).toFixed(2)} TH/s`;
  }
  if (hashesPerSecond < 1_000_000_000) {
    return `${(hashesPerSecond / 1_000_000).toFixed(2)} PH/s`;
  }
  return `${(hashesPerSecond / 1_000_000_000).toFixed(2)} EH/s`;
}

export function formatBTC(satoshis: number): string {
  return (satoshis / 100_000_000).toFixed(8);
}

export function formatNumber(num: number): string {
  return new Intl.NumberFormat('en-US', {
    notation: 'compact',
    maximumFractionDigits: 2,
  }).format(num);
}

export function truncateAddress(address: string, startLength = 6, endLength = 4): string {
  if (address.length <= startLength + endLength) {
    return address;
  }
  return `${address.substring(0, startLength)}...${address.substring(address.length - endLength)}`;
}
