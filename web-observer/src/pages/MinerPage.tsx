import { useState, useEffect } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  fetchMinerStats,
  fetchMinerHashrateHistory,
  formatHashrate,
  formatBTCCompact,
  formatNumber,
  truncateAddress,
  copyToClipboard,
} from '../lib/api';
import type { MinerStats } from '../lib/api';
import { HashrateChart } from '../components/charts/HashrateChart';
import { WorkersTable } from '../components/tables/WorkersTable';
import { EarningsTable } from '../components/tables/EarningsTable';
import { Copy, Check, ChevronLeft } from 'lucide-react';
import { TIME_PERIODS } from '../types';

export function MinerPage() {
  const { address } = useParams<{ address: string }>();
  const [stats, setStats] = useState<MinerStats | null>(null);
  const [hashrateData, setHashrateData] = useState<{ timestamp: string; hashrate: number }[]>([]);
  const [selectedPeriod, setSelectedPeriod] = useState('7d');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    async function loadMinerData() {
      if (!address) return;

      try {
        setLoading(true);
        const [statsData, hashrateHistory] = await Promise.all([
          fetchMinerStats(address),
          fetchMinerHashrateHistory(address, selectedPeriod),
        ]);
        setStats(statsData);
        setHashrateData(hashrateHistory.data_points);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load miner data');
      } finally {
        setLoading(false);
      }
    }
    loadMinerData();
  }, [address, selectedPeriod]);

  const handleCopyAddress = async () => {
    if (!address) return;
    const success = await copyToClipboard(address);
    if (success) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center">
          <div className="spinner mx-auto mb-4" />
          <p className="text-gray-400">Loading miner statistics...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center text-red-400">
          <p className="text-xl mb-2">Error loading miner data</p>
          <p className="text-sm text-gray-400">{error}</p>
        </div>
      </div>
    );
  }

  if (!stats) {
    return null;
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
              <Link to="/pool" className="text-gray-300 hover:text-ocean-primary transition-colors">Pool Stats</Link>
              <Link to="/blocks" className="text-gray-300 hover:text-ocean-primary transition-colors">Blocks</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Back Button */}
        <Link
          to="/"
          className="inline-flex items-center gap-2 text-gray-400 hover:text-ocean-primary transition-colors mb-6"
        >
          <ChevronLeft className="w-4 h-4" />
          Back to Home
        </Link>

        {/* Address Header */}
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-2">
            <h1 className="text-2xl font-bold text-white">Miner Statistics</h1>
          </div>
          <div className="flex items-center gap-2">
            <code className="btc-address text-ocean-primary bg-gray-900 px-3 py-2 rounded-lg">
              {address ? truncateAddress(address, 10, 8) : 'Unknown'}
            </code>
            <button
              onClick={handleCopyAddress}
              className="p-2 text-gray-400 hover:text-ocean-primary transition-colors"
              title="Copy address"
            >
              {copied ? <Check className="w-5 h-5 text-green-400" /> : <Copy className="w-5 h-5" />}
            </button>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <StatCard
            label="Hashrate (3h)"
            value={formatHashrate(stats.hashrate_3h)}
          />
          <StatCard
            label="Shares in Window"
            value={formatNumber(stats.shares_in_window)}
          />
          <StatCard
            label="Est. Reward"
            value={formatBTCCompact(stats.estimated_reward_window)}
          />
          <StatCard
            label="Workers"
            value={stats.workers.length.toString()}
          />
        </div>

        {/* Hashrate History */}
        <div className="bg-ocean-card rounded-xl p-6 border border-gray-800 mb-8">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-ocean-primary">Hashrate History</h2>
            <div className="flex gap-2">
              {TIME_PERIODS.map((period) => (
                <button
                  key={period.value}
                  onClick={() => setSelectedPeriod(period.value)}
                  className={`px-3 py-1 rounded-lg text-sm transition-colors ${
                    selectedPeriod === period.value
                      ? 'bg-ocean-primary text-white'
                      : 'bg-gray-800 text-gray-400 hover:text-white'
                  }`}
                >
                  {period.label}
                </button>
              ))}
            </div>
          </div>
          <HashrateChart data={hashrateData} />
        </div>

        {/* Hashrate Averages */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          {Object.entries(stats.hashrate_avg).map(([period, hashrate]) => (
            <div key={period} className="bg-gray-900 rounded-lg p-4 border border-gray-800">
              <div className="text-gray-400 text-sm mb-1">{period.toUpperCase()} Avg</div>
              <div className="text-lg font-semibold text-white">{formatHashrate(hashrate)}</div>
            </div>
          ))}
        </div>

        {/* Workers Table */}
        <div className="bg-ocean-card rounded-xl p-6 border border-gray-800 mb-8">
          <h2 className="text-xl font-semibold text-ocean-primary mb-6">Workers</h2>
          <WorkersTable workers={stats.workers} />
        </div>

        {/* Latest Earnings */}
        <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
          <h2 className="text-xl font-semibold text-ocean-primary mb-6">Latest Earnings</h2>
          <EarningsTable earnings={stats.latest_earnings} />
        </div>
      </main>
    </div>
  );
}

interface StatCardProps {
  label: string;
  value: string;
}

function StatCard({ label, value }: StatCardProps) {
  return (
    <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
      <div className="text-gray-400 text-sm mb-2">{label}</div>
      <div className="text-2xl font-bold text-white">{value}</div>
    </div>
  );
}
