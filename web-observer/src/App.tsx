import { lazy, Suspense } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Spinner } from './components/ui/Spinner';

// Lazy load pages for code splitting
const HomePage = lazy(() => import('./pages/HomePage').then(m => ({ default: m.HomePage })));
const PoolStatsPage = lazy(() => import('./pages/PoolStatsPage').then(m => ({ default: m.PoolStatsPage })));
const MinerPage = lazy(() => import('./pages/MinerPage').then(m => ({ default: m.MinerPage })));
const BlocksPage = lazy(() => import('./pages/BlocksPage').then(m => ({ default: m.BlocksPage })));
const BlockDetailPage = lazy(() => import('./pages/BlockDetailPage').then(m => ({ default: m.BlockDetailPage })));

// Loading component
function PageLoader() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-950">
      <Spinner />
    </div>
  );
}

function App() {
  return (
    <BrowserRouter>
      <Suspense fallback={<PageLoader />}>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/pool" element={<PoolStatsPage />} />
          <Route path="/miner/:address" element={<MinerPage />} />
          <Route path="/blocks" element={<BlocksPage />} />
          <Route path="/blocks/:height" element={<BlockDetailPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Suspense>
    </BrowserRouter>
  );
}

export default App;
