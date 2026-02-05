import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { fetchBlocks, formatBTCCompact } from '../lib/api';
import type { BlockInfo } from '../lib/api';
import { Blocks, ChevronLeft, ChevronRight, ExternalLink } from 'lucide-react';

export function BlocksPage() {
  const [blocks, setBlocks] = useState<BlockInfo[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const limit = 20;

  useEffect(() => {
    async function loadBlocks() {
      try {
        const data = await fetchBlocks(limit, page * limit);
        setBlocks(data.blocks);
        setTotal(data.total);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load blocks');
      } finally {
        setLoading(false);
      }
    }
    loadBlocks();
  }, [page]);

  const totalPages = Math.ceil(total / limit);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center">
          <div className="spinner mx-auto mb-4" />
          <p className="text-gray-400">Loading blocks...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-950">
        <div className="text-center text-red-400">
          <p className="text-xl mb-2">Error loading blocks</p>
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
              <Link to="/pool" className="text-gray-300 hover:text-ocean-primary transition-colors">Pool Stats</Link>
              <Link to="/blocks" className="text-ocean-primary font-semibold">Blocks</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex items-center justify-between mb-8">
          <h1 className="text-3xl font-bold text-white">Mined Blocks</h1>
          <div className="text-gray-400">
            Total: <span className="text-white font-semibold">{total}</span> blocks
          </div>
        </div>

        {/* Blocks Table */}
        <div className="bg-ocean-card rounded-xl border border-gray-800 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-gray-900 border-b border-gray-800">
                <tr>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Height</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Time</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Reward</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Pool Fee</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Confirmations</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Payouts</th>
                  <th className="px-6 py-4 text-left text-sm font-semibold text-ocean-primary">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-800">
                {blocks.map((block) => (
                  <tr key={block.height} className="hover:bg-gray-900/50 transition-colors">
                    <td className="px-6 py-4">
                      <Link
                        to={`/blocks/${block.height}`}
                        className="flex items-center gap-2 text-white hover:text-ocean-primary transition-colors font-mono"
                      >
                        <Blocks className="w-4 h-4 text-ocean-primary" />
                        #{block.height}
                      </Link>
                    </td>
                    <td className="px-6 py-4 text-gray-400">
                      {new Date(block.time).toLocaleString()}
                    </td>
                    <td className="px-6 py-4 text-white font-semibold">
                      {formatBTCCompact(block.reward_btc)}
                    </td>
                    <td className="px-6 py-4 text-gray-400">{block.pool_fee_percent}%</td>
                    <td className="px-6 py-4">
                      <span className={`px-2 py-1 rounded text-xs font-medium ${
                        block.confirmations >= 6
                          ? 'bg-green-900/50 text-green-400'
                          : 'bg-yellow-900/50 text-yellow-400'
                      }`}>
                        {block.confirmations} confs
                      </span>
                    </td>
                    <td className="px-6 py-4 text-gray-400">{block.payouts_count}</td>
                    <td className="px-6 py-4">
                      <div className="flex gap-2">
                        <Link
                          to={`/blocks/${block.height}`}
                          className="text-ocean-primary hover:text-ocean-secondary transition-colors text-sm"
                        >
                          Details
                        </Link>
                        {block.txid && (
                          <a
                            href={`https://mempool.space/tx/${block.txid}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-gray-400 hover:text-white transition-colors text-sm flex items-center gap-1"
                          >
                            TX <ExternalLink className="w-3 h-3" />
                          </a>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Pagination */}
        {totalPages > 1 && (
          <div className="flex items-center justify-center gap-2 mt-8">
            <button
              onClick={() => setPage(Math.max(0, page - 1))}
              disabled={page === 0}
              className="p-2 rounded-lg bg-gray-800 text-gray-400 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronLeft className="w-5 h-5" />
            </button>
            <div className="text-gray-400">
              Page <span className="text-white font-semibold">{page + 1}</span> of {totalPages}
            </div>
            <button
              onClick={() => setPage(Math.min(totalPages - 1, page + 1))}
              disabled={page >= totalPages - 1}
              className="p-2 rounded-lg bg-gray-800 text-gray-400 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronRight className="w-5 h-5" />
            </button>
          </div>
        )}
      </main>
    </div>
  );
}
