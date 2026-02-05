// API client for DMPool Observer API
//
// Provides typed functions for fetching data from the Observer API

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8082';

export interface PoolStats {
  pool_hashrate_3h: number;
  active_miners: number;
  active_workers: number;
  last_block_height: number;
  next_block_eta_seconds: number;
  pool_fee_percent: number;
  network_difficulty: number;
  block_reward: number;
}

export interface MinerStats {
  address: string;
  shares_in_window: number;
  estimated_reward_window: number;
  estimated_next_block: number;
  hashrate_3h: number;
  hashrate_avg: {
    '1h': number;
    '6h': number;
    '24h': number;
    '7d': number;
  };
  workers: WorkerInfo[];
  latest_earnings: EarningRecord[];
}

export interface WorkerInfo {
  name: string;
  hashrate: number;
  shares: number;
  last_seen: string;
  is_online: boolean;
}

export interface EarningRecord {
  block_height: number;
  time: string;
  amount_btc: number;
  txid: string | null;
  confirmations: number;
}

export interface HashrateDataPoint {
  timestamp: string;
  hashrate: number;
}

export interface BlockInfo {
  height: number;
  time: string;
  reward_btc: number;
  pool_fee_percent: number;
  txid: string | null;
  confirmations: number;
  payouts_count: number;
}

export interface BlockDetail {
  height: number;
  time: string;
  reward_btc: number;
  pool_fee_btc: number;
  network_difficulty: number;
  txid: string | null;
  confirmations: number;
  pplns_window_shares: number;
  payouts: PayoutDetail[];
}

export interface PayoutDetail {
  address: string;
  amount_btc: number;
  shares: number;
  share_percent: number;
}

// API functions
export async function fetchPoolStats(): Promise<PoolStats> {
  const response = await fetch(`${API_BASE_URL}/api/v1/stats`);
  if (!response.ok) {
    throw new Error(`Failed to fetch pool stats: ${response.statusText}`);
  }
  return response.json();
}

export async function fetchMinerStats(address: string): Promise<MinerStats> {
  const response = await fetch(`${API_BASE_URL}/api/v1/stats/${address}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch miner stats: ${response.statusText}`);
  }
  return response.json();
}

export async function fetchMinerHashrateHistory(
  address: string,
  period: string = '7d'
): Promise<{ data_points: HashrateDataPoint[] }> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/stats/${address}/hashrate?period=${period}`
  );
  if (!response.ok) {
    throw new Error(`Failed to fetch hashrate history: ${response.statusText}`);
  }
  return response.json();
}

export async function fetchBlocks(limit: number = 20, offset: number = 0): Promise<{
  total: number;
  blocks: BlockInfo[];
}> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/blocks?limit=${limit}&offset=${offset}`
  );
  if (!response.ok) {
    throw new Error(`Failed to fetch blocks: ${response.statusText}`);
  }
  return response.json();
}

export async function fetchBlockDetail(height: number): Promise<BlockDetail> {
  const response = await fetch(`${API_BASE_URL}/api/v1/blocks/${height}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch block detail: ${response.statusText}`);
  }
  return response.json();
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

export function formatBTCCompact(satoshis: number): string {
  const btc = satoshis / 100_000_000;
  if (btc >= 1) {
    return `${btc.toFixed(4)} BTC`;
  }
  if (btc >= 0.001) {
    return `${(btc * 1000).toFixed(2)} mBTC`;
  }
  return `${(btc * 1_000_000).toFixed(2)} µBTC`;
}

export function formatNumber(num: number): string {
  return new Intl.NumberFormat('en-US', {
    notation: 'compact',
    maximumFractionDigits: 2,
  }).format(num);
}

export function truncateAddress(address: string, startLength: number = 6, endLength: number = 4): string {
  if (address.length <= startLength + endLength) {
    return address;
  }
  return `${address.substring(0, startLength)}...${address.substring(address.length - endLength)}`;
}

export function copyToClipboard(text: string): Promise<boolean> {
  if (navigator.clipboard) {
    return navigator.clipboard.writeText(text)
      .then(() => true)
      .catch(() => false);
  }
  // Fallback for older browsers
  const textArea = document.createElement('textarea');
  textArea.value = text;
  textArea.style.position = 'fixed';
  textArea.style.left = '-9999px';
  document.body.appendChild(textArea);
  textArea.select();
  try {
    document.execCommand('copy');
    document.body.removeChild(textArea);
    return Promise.resolve(true);
  } catch {
    document.body.removeChild(textArea);
    return Promise.resolve(false);
  }
}
