import { useState, useEffect } from 'react';
import { Link, useParams } from 'react-router-dom';
import { fetchBlockDetail, formatBTC, formatNumber, truncateAddress } from '../lib/api';
import type { BlockDetail } from '../lib/api';
import { Blocks, ChevronLeft, ExternalLink, Users } from 'lucide-react';

export function BlockDetailPage() {
  const { height } = useParams<{ height: string }>();
  const [block, setBlock] = useState<BlockDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadBlockDetail() {
      if (!height) return;

      try {
        const data = await fetchBlockDetail(parseInt(height));
        setBlock(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load block details');
      } finally {
        setLoading(false);
      }
    }
    loadBlockDetail();
  }, [height]);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center">
          <div className="spinner mx-auto mb-4" />
          <p className="text-gray-400">Loading block details...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center text-red-400">
          <p className="text-xl mb-2">Error loading block details</p>
          <p className="text-sm text-gray-400">{error}</p>
        </div>
      </div>
    );
  }

  if (!block) {
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
          to="/blocks"
          className="inline-flex items-center gap-2 text-gray-400 hover:text-ocean-primary transition-colors mb-6"
        >
          <ChevronLeft className="w-4 h-4" />
          Back to Blocks
        </Link>

        {/* Block Header */}
        <div className="flex items-center gap-3 mb-8">
          <Blocks className="w-8 h-8 text-ocean-primary" />
          <h1 className="text-3xl font-bold text-white">Block #{block.height}</h1>
          {block.txid && (
            <a
              href={`https://mempool.space/tx/${block.txid}`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-ocean-primary transition-colors flex items-center gap-1"
            >
              View Transaction <ExternalLink className="w-4 h-4" />
            </a>
          )}
        </div>

        {/* Block Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <StatCard
            label="Reward"
            value={`${formatBTC(block.reward_btc)} BTC`}
          />
          <StatCard
            label="Pool Fee"
            value={`${formatBTC(block.pool_fee_btc)} BTC`}
          />
          <StatCard
            label="Confirmations"
            value={block.confirmations.toString()}
          />
          <StatCard
            label="PPLNS Window Shares"
            value={formatNumber(block.pplns_window_shares)}
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Block Info */}
          <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
            <h2 className="text-xl font-semibold text-ocean-primary mb-4">Block Information</h2>
            <div className="space-y-4">
              <InfoRow label="Height" value={`#${block.height}`} />
              <InfoRow label="Time" value={new Date(block.time).toLocaleString()} />
              <InfoRow label="Network Difficulty" value={formatNumber(block.network_difficulty)} />
              <InfoRow
                label="Transaction ID"
                value={
                  block.txid ? (
                    <a
                      href={`https://mempool.space/tx/${block.txid}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="btc-address text-ocean-primary hover:text-ocean-secondary transition-colors flex items-center gap-1"
                    >
                      {truncateAddress(block.txid, 8, 8)}
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  ) : (
                    'Pending...'
                  )
                }
              />
            </div>
          </div>

          {/* Confirmations Status */}
          <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
            <h2 className="text-xl font-semibold text-ocean-primary mb-4">Confirmation Status</h2>
            <div className="space-y-4">
              <div className="flex justify-between items-center">
                <span className="text-gray-400">Confirmations</span>
                <span className={`text-2xl font-bold ${
                  block.confirmations >= 6 ? 'text-green-400' : 'text-yellow-400'
                }`}>
                  {block.confirmations}
                </span>
              </div>
              <div className="w-full bg-gray-800 rounded-full h-3">
                <div
                  className={`h-3 rounded-full transition-all ${
                    block.confirmations >= 6 ? 'bg-green-500' : 'bg-yellow-500'
                  }`}
                  style={{ width: `${Math.min(100, (block.confirmations / 6) * 100)}%` }}
                />
              </div>
              <p className="text-sm text-gray-500">
                {block.confirmations >= 6
                  ? 'Block is fully confirmed'
                  : `${6 - block.confirmations} more confirmation(s) needed`}
              </p>
            </div>
          </div>
        </div>

        {/* Payouts */}
        <div className="bg-ocean-card rounded-xl p-6 border border-gray-800">
          <div className="flex items-center gap-2 mb-6">
            <Users className="w-6 h-6 text-ocean-primary" />
            <h2 className="text-xl font-semibold text-ocean-primary">Payouts</h2>
            <span className="text-gray-400">({block.payouts.length} miners)</span>
          </div>

          {block.payouts.length === 0 ? (
            <p className="text-gray-400 text-center py-8">No payouts yet - block awaiting confirmation</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-gray-900 border-b border-gray-800">
                  <tr>
                    <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Miner Address</th>
                    <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Shares</th>
                    <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Share %</th>
                    <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Reward</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-800">
                  {block.payouts.map((payout, index) => (
                    <tr key={index} className="hover:bg-gray-900/50 transition-colors">
                      <td className="px-4 py-3">
                        <code className="btc-address text-ocean-primary text-sm">
                          {truncateAddress(payout.address, 10, 8)}
                        </code>
                      </td>
                      <td className="px-4 py-3 text-right text-gray-400 font-mono">
                        {formatNumber(payout.shares)}
                      </td>
                      <td className="px-4 py-3 text-right text-gray-400">
                        {payout.share_percent.toFixed(2)}%
                      </td>
                      <td className="px-4 py-3 text-right text-white font-semibold">
                        {formatBTC(payout.amount_btc)} BTC
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
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

interface InfoRowProps {
  label: string;
  value: React.ReactNode;
}

function InfoRow({ label, value }: InfoRowProps) {
  return (
    <div className="flex justify-between items-center py-2">
      <span className="text-gray-400">{label}</span>
      <span className="text-white text-right">{value}</span>
    </div>
  );
}
