// Type definitions for DMPool Observer
//
// Shared types used across the application

export type { WorkerInfo, EarningRecord, HashrateDataPoint, BlockInfo } from '../lib/api';

export interface Route {
  path: string;
  component: React.LazyExoticComponent<React.ComponentType>;
}

export interface SearchParams {
  address?: string;
}

export interface ChartDataPoint {
  timestamp: string;
  hashrate: number;
}

export interface StatsOverviewProps {
  sharesInWindow: string;
  estimatedRewardWindow: string;
  estimatedNextBlock: string;
}

export interface TimeRange {
  label: string;
  value: string;
}

export const TIME_RANGES: TimeRange[] = [
  { label: '1W', value: '1w' },
  { label: '1M', value: '1m' },
  { label: '6M', value: '6m' },
  { label: 'ALL', value: 'all' },
];

export const TIME_PERIODS = [
  { label: '1天', value: '1d' },
  { label: '3天', value: '3d' },
  { label: '7天', value: '7d' },
  { label: '30天', value: '30d' },
];
