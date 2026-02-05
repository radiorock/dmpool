import { useMemo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { formatHashrate } from '../../lib/api';

interface HashrateChartProps {
  data: Array<{ timestamp: string; hashrate: number }>;
}

export function HashrateChart({ data }: HashrateChartProps) {
  const chartData = useMemo(() => {
    return data.map((point) => ({
      time: new Date(point.timestamp).toLocaleTimeString('en-US', {
        hour: '2-digit',
        minute: '2-digit',
      }),
      hashrate: point.hashrate,
    }));
  }, [data]);

  const formatYAxis = (value: number) => formatHashrate(value);

  const formatTooltip = (value?: number) => [value !== undefined ? formatHashrate(value) : 'N/A', 'Hashrate'];

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-400">
        No data available for the selected period
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
        <XAxis
          dataKey="time"
          stroke="#9ca3af"
          tick={{ fill: '#9ca3af' }}
        />
        <YAxis
          tickFormatter={formatYAxis}
          stroke="#9ca3af"
          tick={{ fill: '#9ca3af' }}
          width={80}
        />
        <Tooltip
          contentStyle={{
            backgroundColor: '#1f2937',
            border: '1px solid #374151',
            borderRadius: '0.5rem',
          }}
          formatter={formatTooltip}
          labelStyle={{ color: '#f3f4f6' }}
        />
        <Line
          type="monotone"
          dataKey="hashrate"
          stroke="#38bdf8"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 6, stroke: '#38bdf8', strokeWidth: 2, fill: '#0a0e1a' }}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}
