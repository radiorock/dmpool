<script setup lang="ts">
import { ref, onMounted } from 'vue';
import {
  NCard,
  NGrid,
  NGridItem,
  NStatistic,
  NSpin,
  NAlert,
  NIcon,
} from 'naive-ui';
import {
  ServerOutline,
  PeopleOutline,
  HardwareChipOutline,
  CubeOutline,
} from '@vicons/ionicons5';
import { fetchDashboardStats, formatHashrate } from '../api';
import type { PoolStats } from '../types';

const loading = ref(true);
const error = ref<string | null>(null);
const stats = ref<PoolStats | null>(null);

async function loadStats() {
  try {
    loading.value = true;
    error.value = null;
    stats.value = await fetchDashboardStats();
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load dashboard stats';
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  loadStats();
  // Refresh every 30 seconds
  setInterval(loadStats, 30000);
});
</script>

<template>
  <div class="dashboard-view">
    <div class="mb-6">
      <h2 class="text-2xl font-bold text-white">Dashboard</h2>
      <p class="text-gray-400">Pool overview and statistics</p>
    </div>

    <NSpin :show="loading">
      <NAlert v-if="error" type="error" class="mb-4">
        {{ error }}
      </NAlert>

      <div v-if="stats">
        <NGrid :cols="1" :x-gap="16" :y-gap="16" responsive="screen">
          <NGridItem :span="1">
            <NCard>
              <NStatistic label="Pool Hashrate" :value="formatHashrate(stats.pool_hashrate)">
                <template #prefix>
                  <NIcon :component="ServerOutline" class="text-ocean-primary" />
                </template>
              </NStatistic>
            </NCard>
          </NGridItem>

          <NGridItem :span="1">
            <NCard>
              <NStatistic label="Total Miners" :value="stats.total_miners">
                <template #prefix>
                  <NIcon :component="PeopleOutline" class="text-ocean-primary" />
                </template>
              </NStatistic>
            </NCard>
          </NGridItem>

          <NGridItem :span="1">
            <NCard>
              <NStatistic label="Total Workers" :value="stats.total_workers">
                <template #prefix>
                  <NIcon :component="HardwareChipOutline" class="text-ocean-primary" />
                </template>
              </NStatistic>
            </NCard>
          </NGridItem>

          <NGridItem :span="1">
            <NCard>
              <NStatistic label="Last Block" :value="`#${stats.last_block_height}`">
                <template #prefix>
                  <NIcon :component="CubeOutline" class="text-ocean-primary" />
                </template>
              </NStatistic>
            </NCard>
          </NGridItem>
        </NGrid>

        <NCard class="mt-4">
          <h3 class="text-lg font-semibold text-white mb-2">Next Block ETA</h3>
          <p class="text-3xl font-bold text-ocean-primary">
            {{ Math.floor(stats.next_block_eta / 3600) }}h {{ Math.floor((stats.next_block_eta % 3600) / 60) }}m
          </p>
        </NCard>
      </div>
    </NSpin>
  </div>
</template>

<style scoped>
.dashboard-view {
  max-width: 1400px;
  margin: 0 auto;
}

:deep(.n-statistic .n-statistic__label) {
  color: #94a3b8;
}

:deep(.n-statistic .n-statistic__value) {
  color: #f1f5f9;
}

:deep(.n-card) {
  background-color: #1e293b !important;
  border-color: #334155 !important;
}
</style>
