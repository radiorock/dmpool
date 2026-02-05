import type { WorkerInfo } from '../../lib/api';
import { formatHashrate, formatNumber } from '../../lib/api';
import { Activity } from 'lucide-react';

interface WorkersTableProps {
  workers: WorkerInfo[];
}

export function WorkersTable({ workers }: WorkersTableProps) {
  if (workers.length === 0) {
    return (
      <div className="text-center py-8 text-gray-400">
        No active workers
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead className="bg-gray-900 border-b border-gray-800">
          <tr>
            <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Worker</th>
            <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Hashrate</th>
            <th className="px-4 py-3 text-right text-sm font-semibold text-ocean-primary">Shares</th>
            <th className="px-4 py-3 text-left text-sm font-semibold text-ocean-primary">Last Seen</th>
            <th className="px-4 py-3 text-center text-sm font-semibold text-ocean-primary">Status</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-800">
          {workers.map((worker) => (
            <tr key={worker.name} className="hover:bg-gray-900/50 transition-colors">
              <td className="px-4 py-3">
                <div className="flex items-center gap-2">
                  <Activity className={`w-4 h-4 ${worker.is_online ? 'text-green-400' : 'text-gray-500'}`} />
                  <span className="text-white font-medium">{worker.name}</span>
                </div>
              </td>
              <td className="px-4 py-3 text-right text-white font-mono">
                {formatHashrate(worker.hashrate)}
              </td>
              <td className="px-4 py-3 text-right text-gray-400 font-mono">
                {formatNumber(worker.shares)}
              </td>
              <td className="px-4 py-3 text-gray-400 text-sm">
                {new Date(worker.last_seen).toLocaleString()}
              </td>
              <td className="px-4 py-3 text-center">
                <span
                  className={`inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium ${
                    worker.is_online
                      ? 'bg-green-900/50 text-green-400'
                      : 'bg-gray-800 text-gray-500'
                  }`}
                >
                  <span
                    className={`w-2 h-2 rounded-full ${
                      worker.is_online ? 'bg-green-400 animate-pulse' : 'bg-gray-500'
                    }`}
                  />
                  {worker.is_online ? 'Online' : 'Offline'}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
