<script setup lang="ts">
import { ref, onMounted, h } from 'vue';
import {
  NCard,
  NDataTable,
  NButton,
  NSpace,
  NSpin,
  NAlert,
  NInput,
  NModal,
  NInputNumber,
  NTag,
  NPopconfirm,
  useDialog,
} from 'naive-ui';
import type { DataTableColumns } from 'naive-ui';
import { fetchMiners, banMiner, unbanMiner, updateMinerThreshold, truncateAddress, formatHashrate } from '../api';
import type { MinerInfo } from '../types';

const dialog = useDialog();
const loading = ref(true);
const error = ref<string | null>(null);
const miners = ref<MinerInfo[]>([]);
const total = ref(0);
const searchQuery = ref('');
const pageSize = 20;
const currentPage = ref(1);

// Threshold modal
const showThresholdModal = ref(false);
const selectedMiner = ref<MinerInfo | null>(null);
const newThreshold = ref<number>(0);

async function loadMiners() {
  try {
    loading.value = true;
    error.value = null;
    const response = await fetchMiners({
      search: searchQuery.value || undefined,
      limit: pageSize,
      offset: (currentPage.value - 1) * pageSize,
    });
    miners.value = response.miners;
    total.value = response.total;
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load miners';
  } finally {
    loading.value = false;
  }
}

async function handleBan(miner: MinerInfo) {
  try {
    await banMiner(miner.address);
    await loadMiners();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to ban miner',
    });
  }
}

async function handleUnban(miner: MinerInfo) {
  try {
    await unbanMiner(miner.address);
    await loadMiners();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to unban miner',
    });
  }
}

function openThresholdModal(miner: MinerInfo) {
  selectedMiner.value = miner;
  newThreshold.value = miner.custom_threshold || 0;
  showThresholdModal.value = true;
}

async function handleUpdateThreshold() {
  if (!selectedMiner.value) return;
  try {
    await updateMinerThreshold(selectedMiner.value.address, newThreshold.value);
    showThresholdModal.value = false;
    await loadMiners();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to update threshold',
    });
  }
}

const columns: DataTableColumns<MinerInfo> = [
  {
    title: 'Address',
    key: 'address',
    render: (row) => truncateAddress(row.address, 8, 8),
  },
  {
    title: 'Hashrate',
    key: 'hashrate',
    render: (row) => formatHashrate(row.hashrate),
  },
  {
    title: 'Shares',
    key: 'shares',
    render: (row) => row.shares.toLocaleString(),
  },
  {
    title: 'Workers',
    key: 'workers',
    render: (row) => row.workers.toString(),
  },
  {
    title: 'Status',
    key: 'banned',
    render: (row) =>
      row.banned
        ? h(NTag, { type: 'error' }, { default: () => 'Banned' })
        : h(NTag, { type: 'success' }, { default: () => 'Active' }),
  },
  {
    title: 'Actions',
    key: 'actions',
    render: (row) =>
      h(NSpace, null, {
        default: () => [
          row.banned
            ? h(
                NPopconfirm,
                {
                  onPositiveClick: () => handleUnban(row),
                },
                {
                  default: () => 'Unban this miner?',
                  trigger: () =>
                    h(NButton, { size: 'small', type: 'success' }, {
                      default: () => 'Unban',
                    }),
                }
              )
            : h(
                NPopconfirm,
                {
                  onPositiveClick: () => handleBan(row),
                },
                {
                  default: () => 'Ban this miner?',
                  trigger: () =>
                    h(NButton, { size: 'small', type: 'error' }, {
                      default: () => 'Ban',
                    }),
                }
              ),
          h(NButton, {
            size: 'small',
            onClick: () => openThresholdModal(row),
          }, { default: () => 'Threshold' }),
        ],
      }),
  },
];

onMounted(() => {
  loadMiners();
});

function handleSearch() {
  currentPage.value = 1;
  loadMiners();
}
</script>

<template>
  <div class="miners-view">
    <div class="mb-6 flex justify-between items-center">
      <div>
        <h2 class="text-2xl font-bold text-white">Miners</h2>
        <p class="text-gray-400">Manage pool miners</p>
      </div>
      <NInput
        v-model:value="searchQuery"
        placeholder="Search by address..."
        style="width: 300px"
        @keyup.enter="handleSearch"
      />
    </div>

    <NSpin :show="loading">
      <NAlert v-if="error" type="error" class="mb-4">
        {{ error }}
      </NAlert>

      <NCard>
        <NDataTable
          :columns="columns"
          :data="miners"
          :pagination="{
            page: currentPage,
            pageSize: pageSize,
            pageCount: Math.ceil(total / pageSize),
            onChange: (page: number) => {
              currentPage = page;
              loadMiners();
            },
          }"
          :bordered="false"
        />
      </NCard>
    </NSpin>

    <!-- Threshold Modal -->
    <NModal
      v-model:show="showThresholdModal"
      preset="card"
      title="Update Payout Threshold"
      style="width: 400px"
    >
      <div class="py-4">
        <p class="mb-4 text-gray-300">
          Set custom payout threshold (in satoshis) for:
        </p>
        <p class="mb-4 font-mono text-sm text-ocean-primary">
          {{ selectedMiner ? truncateAddress(selectedMiner.address, 10, 8) : '' }}
        </p>
        <NInputNumber
          v-model:value="newThreshold"
          :min="0"
          placeholder="0 = default threshold"
          style="width: 100%"
        />
      </div>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="showThresholdModal = false">Cancel</NButton>
          <NButton type="primary" @click="handleUpdateThreshold">Update</NButton>
        </NSpace>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.miners-view {
  max-width: 1400px;
  margin: 0 auto;
}

:deep(.n-card) {
  background-color: #1e293b !important;
  border-color: #334155 !important;
}
</style>
