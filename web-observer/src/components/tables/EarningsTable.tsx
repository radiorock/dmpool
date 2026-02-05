import type { EarningRecord } from '../../lib/api';
import { formatBTC, truncateAddress } from '../../lib/api';
import { ExternalLink } from 'lucide-react';

interface EarningsTableProps {
  earnings: EarningRecord[];
}

export function EarningsTable({ earnings }: EarningsTableProps) {
  if (earnings.length === 0) {
    return (
      <div className="text-center py-8 text-gray-400">
        No earnings yet. Keep mining!
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead className="bg-gray-900 border-b border-gray-800">
          <tr>
            <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Block</th>
            <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Time</th>
            <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Amount</th>
            <th className="px-4 py-3 text-center text-sm font-semibold text-ocean-primary">Confirmations</th>
            <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Transaction</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-800">
          {earnings.map((earning) => (
            <tr key={earning.block_height} className="hover:bg-gray-900/50 transition-colors">
              <td className="px-4 py-3">
                <span className="text-white font-medium">#{earning.block_height}</span>
              </td>
              <td className="px-4 py-3 text-gray-400 text-sm">
                {new Date(earning.time).toLocaleString()}
              </td>
              <td className="px-4 py-3 text-right text-white font-semibold font-mono">
                {formatBTC(earning.amount_btc)} BTC
              </td>
              <td className="px-4 py-3 text-center">
                <span
                  className={`inline-flex px-2 py-1 rounded-full text-xs font-medium ${
                    earning.confirmations >= 6
                      ? 'bg-green-900/50 text-green-400'
                      : earning.txid
                      ? 'bg-yellow-900/50 text-yellow-400'
                      : 'bg-gray-800 text-gray-500'
                  }`}
                >
                  {earning.confirmations} confs
                </span>
              </td>
              <td className="px-4 py-3">
                {earning.txid ? (
                  <a
                    href={`https://mempool.space/tx/${earning.txid}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-ocean-primary hover:text-ocean-secondary transition-colors flex items-center gap-1 text-sm"
                  >
                    {truncateAddress(earning.txid, 8, 8)}
                    <ExternalLink className="w-3 h-3" />
                  </a>
                ) : (
                  <span className="text-gray-500 text-sm">Pending...</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
