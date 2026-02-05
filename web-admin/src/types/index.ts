// Type definitions for DMPool Admin Panel

export interface PoolStats {
  total_miners: number;
  total_workers: number;
  pool_hashrate: number;
  last_block_height: number;
  next_block_eta: number;
}

export interface MinerInfo {
  address: string;
  hashrate: number;
  shares: number;
  workers: number;
  banned: boolean;
  custom_threshold?: number;
}

export interface PaymentInfo {
  txid: string;
  amount: number;
  fee: number;
  status: 'pending' | 'broadcast' | 'confirmed' | 'failed';
  timestamp: string;
}

export interface NotificationConfig {
  id: number;
  type: 'telegram' | 'email';
  enabled: boolean;
  config: Record<string, unknown>;
}

export interface SystemConfig {
  key: string;
  value: string;
  description: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}
