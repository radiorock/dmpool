import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { fetchPoolStats, formatHashrate, formatBTC, formatNumber } from '../lib/api';
import type { PoolStats } from '../lib/api';
import { BarChart3, Users, HardDrive, Clock, Activity } from 'lucide-react';

export function PoolStatsPage() {
  const [stats, setStats] = useState<PoolStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadStats() {
      try {
        const data = await fetchPoolStats();
        setStats(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load pool stats');
      } finally {
        setLoading(false);
      }
    }
    loadStats();
    // Refresh every 30 seconds
    const interval = setInterval(loadStats, 30000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center">
          <div className="spinner mx-auto mb-4" />
          <p className="text-gray-400">Loading pool statistics...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center text-red-400">
          <p className="text-xl mb-2">Error loading pool data</p>
          <p className="text-sm text-gray-400">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-950">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-950/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <Link to="/" className="text-2xl font-bold text-ocean-primary hover:text-ocean-secondary transition-colors">
              DMPool Observer
            </Link>
            <nav className="flex gap-6">
              <Link to="/" className="text-gray-300 hover:text-ocean-primary transition-colors">Home</Link>
              <Link to="/pool" className="text-ocean-primary font-semibold">Pool Stats</Link>
              <Link to="/blocks" className="text-gray-300 hover:text-ocean-primary transition-colors">Blocks</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <h1 className="text-3xl font-bold text-white mb-8">Pool Statistics</h1>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
          <StatCard
            icon={<BarChart3 className="w-6 h-6" />}
            label="Pool Hashrate (3h avg)"
            value={formatHashrate(stats!.pool_hashrate_3h)}
          />
          <StatCard
            icon={<Users className="w-6 h-6" />}
            label="Active Miners"
            value={formatNumber(stats!.active_miners)}
          />
          <StatCard
            icon={<HardDrive className="w-6 h-6" />}
            label="Active Workers"
            value={formatNumber(stats!.active_workers)}
          />
          <StatCard
            icon={<Activity className="w-6 h-6" />}
            label="Last Block Height"
            value={`#${stats!.last_block_height}`}
          />
          <StatCard
            icon={<Clock className="w-6 h-6" />}
            label="Next Block ETA"
            value={formatTime(stats!.next_block_eta_seconds)}
          />
          <StatCard
            icon={<HardDrive className="w-6 h-6" />}
            label="Network Difficulty"
            value={formatNumber(stats!.network_difficulty)}
          />
        </div>

        {/* Reward Info */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
            <h3 className="text-lg font-semibold text-ocean-primary mb-4">Block Reward</h3>
            <div className="text-3xl font-bold text-white mb-2">
              {formatBTC(stats!.block_reward)} BTC
            </div>
          </div>
          <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
            <h3 className="text-lg font-semibold text-ocean-primary mb-4">Pool Fee</h3>
            <div className="text-3xl font-bold text-white mb-2">
              {stats!.pool_fee_percent}%
            </div>
            <p className="text-gray-400 text-sm">Fee supports pool infrastructure and development</p>
          </div>
        </div>
      </main>
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
}

function StatCard({ icon, label, value }: StatCardProps) {
  return (
    <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
      <div className="flex items-center gap-3 mb-3">
        <div className="text-ocean-primary">{icon}</div>
        <span className="text-gray-400 text-sm">{label}</span>
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
    </div>
  );
}

function formatTime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 24) {
    const days = Math.floor(hours / 24);
    return `${days}d ${hours % 24}h`;
  }
  return `${hours}h ${minutes}m`;
}
