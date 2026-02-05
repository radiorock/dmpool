import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { fetchPoolStats, formatHashrate, formatBTCCompact, formatNumber } from '../lib/api';
import type { PoolStats } from '../lib/api';
import { BarChart3, Users, HardDrive, Clock, TrendingUp } from 'lucide-react';

export function HomePage() {
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
    <div className="min-h-screen bg-gradient-to-br from-gray-950 via-ocean-dark to-gray-950">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-950/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold text-ocean-primary">DMPool Observer</h1>
            <nav className="flex gap-6">
              <Link to="/" className="text-gray-300 hover:text-ocean-primary transition-colors">Home</Link>
              <Link to="/pool" className="text-gray-300 hover:text-ocean-primary transition-colors">Pool Stats</Link>
              <Link to="/blocks" className="text-gray-300 hover:text-ocean-primary transition-colors">Blocks</Link>
            </nav>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="py-16 px-4">
        <div className="max-w-4xl mx-auto text-center">
          <h2 className="text-4xl md:text-5xl font-bold mb-6 bg-gradient-to-r from-ocean-primary to-ocean-secondary bg-clip-text text-transparent">
            Decentralized Mining Pool
          </h2>
          <p className="text-xl text-gray-400 mb-8">
            Non-custodial PPLNS mining pool powered by Bitcoin
          </p>
          <div className="flex justify-center gap-4">
            <a
              href="https://github.com/dmpool/dmpool"
              target="_blank"
              rel="noopener noreferrer"
              className="px-6 py-3 bg-ocean-primary hover:bg-ocean-600 rounded-lg font-semibold transition-colors"
            >
              GitHub
            </a>
            <Link
              to="/pool"
              className="px-6 py-3 border border-ocean-primary text-ocean-primary hover:bg-ocean-primary/10 rounded-lg font-semibold transition-colors"
            >
              View Stats
            </Link>
          </div>
        </div>
      </section>

      {/* Stats Grid */}
      {stats && (
        <section className="py-8 px-4">
          <div className="max-w-7xl mx-auto">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
              {/* Pool Hashrate */}
              <StatCard
                icon={<BarChart3 className="w-6 h-6" />}
                label="Pool Hashrate"
                value={formatHashrate(stats.pool_hashrate_3h)}
                trend="+2.4%"
              />

              {/* Active Miners */}
              <StatCard
                icon={<Users className="w-6 h-6" />}
                label="Active Miners"
                value={formatNumber(stats.active_miners)}
                subtitle={`${formatNumber(stats.active_workers)} workers`}
              />

              {/* Block Reward */}
              <StatCard
                icon={<HardDrive className="w-6 h-6" />}
                label="Block Reward"
                value={formatBTCCompact(stats.block_reward)}
                subtitle={`${stats.pool_fee_percent}% pool fee`}
              />

              {/* Next Block ETA */}
              <StatCard
                icon={<Clock className="w-6 h-6" />}
                label="Next Block ETA"
                value={formatTime(stats.next_block_eta_seconds)}
                subtitle={`Height: ${stats.last_block_height + 1}`}
              />
            </div>
          </div>
        </section>
      )}

      {/* Features Section */}
      <section className="py-16 px-4">
        <div className="max-w-6xl mx-auto">
          <h3 className="text-2xl font-bold text-center mb-12 text-ocean-primary">Why DMPool?</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            <FeatureCard
              title="Non-Custodial"
              description="Your miners, your rewards. Payouts go directly to your mining address."
              icon={<TrendingUp className="w-8 h-8" />}
            />
            <FeatureCard
              title="PPLNS"
              description="Fair Pay Per Last N Shares system ensures proportional rewards."
              icon={<BarChart3 className="w-8 h-8" />}
            />
            <FeatureCard
              title="Decentralized"
              description="Based on OCEAN.xyz model - no registration, address-as-identity."
              icon={<Users className="w-8 h-8" />}
            />
          </div>
        </div>
      </section>

      {/* Miner Search */}
      <section className="py-16 px-4 bg-ocean-dark/50">
        <div className="max-w-xl mx-auto text-center">
          <h3 className="text-xl font-semibold mb-4">Check Your Miner Stats</h3>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              const formData = new FormData(e.currentTarget);
              const address = formData.get('address') as string;
              if (address) {
                window.location.href = `/miner/${address}`;
              }
            }}
            className="flex gap-2"
          >
            <input
              type="text"
              name="address"
              placeholder="Enter Bitcoin address..."
              className="flex-1 px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-ocean-primary text-white"
              required
            />
            <button
              type="submit"
              className="px-6 py-3 bg-ocean-primary hover:bg-ocean-600 rounded-lg font-semibold transition-colors"
            >
              Search
            </button>
          </form>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-gray-800 py-8 px-4">
        <div className="max-w-7xl mx-auto text-center text-gray-500 text-sm">
          <p>&copy; 2025 DMPool. Decentralized Bitcoin Mining Pool.</p>
        </div>
      </footer>
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  trend?: string;
  subtitle?: string;
}

function StatCard({ icon, label, value, trend, subtitle }: StatCardProps) {
  return (
    <div className="bg-ocean-card rounded-xl p-6 border border-gray-800 hover:border-ocean-primary/50 transition-colors">
      <div className="flex items-center gap-3 mb-4">
        <div className="text-ocean-primary">{icon}</div>
        <span className="text-gray-400 text-sm">{label}</span>
      </div>
      <div className="text-2xl font-bold text-white mb-1">{value}</div>
      {trend && (
        <div className="text-sm text-green-400">{trend}</div>
      )}
      {subtitle && (
        <div className="text-sm text-gray-500">{subtitle}</div>
      )}
    </div>
  );
}

interface FeatureCardProps {
  title: string;
  description: string;
  icon: React.ReactNode;
}

function FeatureCard({ title, description, icon }: FeatureCardProps) {
  return (
    <div className="bg-ocean-card rounded-xl p-6 border border-gray-800 text-center">
      <div className="text-ocean-primary mx-auto mb-4">{icon}</div>
      <h4 className="text-lg font-semibold mb-2">{title}</h4>
      <p className="text-gray-400 text-sm">{description}</p>
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
