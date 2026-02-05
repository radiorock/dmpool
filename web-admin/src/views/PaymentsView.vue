<script setup lang="ts">
import { ref, onMounted, h } from 'vue';
import {
  NCard,
  NDataTable,
  NButton,
  NSpace,
  NSpin,
  NAlert,
  NTag,
  NPopconfirm,
  useDialog,
  NSelect,
} from 'naive-ui';
import type { DataTableColumns } from 'naive-ui';
import {
  SendOutline,
} from '@vicons/ionicons5';
import { fetchPayments, triggerPayment, formatBTC, truncateAddress } from '../api';
import type { PaymentInfo } from '../types';

const dialog = useDialog();
const loading = ref(true);
const triggering = ref(false);
const error = ref<string | null>(null);
const payments = ref<PaymentInfo[]>([]);
const total = ref(0);
const statusFilter = ref<string | null>(null);
const pageSize = 20;
const currentPage = ref(1);

const statusOptions = [
  { label: 'All', value: '' },
  { label: 'Pending', value: 'pending' },
  { label: 'Broadcast', value: 'broadcast' },
  { label: 'Confirmed', value: 'confirmed' },
  { label: 'Failed', value: 'failed' },
];

async function loadPayments() {
  try {
    loading.value = true;
    error.value = null;
    const response = await fetchPayments({
      status: statusFilter.value || undefined,
      limit: pageSize,
      offset: (currentPage.value - 1) * pageSize,
    });
    payments.value = response.payments;
    total.value = response.total;
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load payments';
  } finally {
    loading.value = false;
  }
}

async function handleTriggerPayment() {
  try {
    triggering.value = true;
    await triggerPayment();
    dialog.success({
      title: 'Success',
      content: 'Payment triggered successfully',
    });
    await loadPayments();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to trigger payment',
    });
  } finally {
    triggering.value = false;
  }
}

const columns: DataTableColumns<PaymentInfo> = [
  {
    title: 'Transaction ID',
    key: 'txid',
    render: (row) => truncateAddress(row.txid, 10, 8),
  },
  {
    title: 'Amount (BTC)',
    key: 'amount',
    render: (row) => formatBTC(row.amount),
  },
  {
    title: 'Fee (BTC)',
    key: 'fee',
    render: (row) => formatBTC(row.fee),
  },
  {
    title: 'Status',
    key: 'status',
    render: (row) => {
      const statusMap: Record<string, 'default' | 'info' | 'success' | 'error'> = {
        pending: 'default',
        broadcast: 'info',
        confirmed: 'success',
        failed: 'error',
      };
      const labelMap: Record<string, string> = {
        pending: 'Pending',
        broadcast: 'Broadcast',
        confirmed: 'Confirmed',
        failed: 'Failed',
      };
      return h(NTag, { type: statusMap[row.status] || 'default' }, {
        default: () => labelMap[row.status] || row.status,
      });
    },
  },
  {
    title: 'Timestamp',
    key: 'timestamp',
    render: (row) => new Date(row.timestamp).toLocaleString(),
  },
];

onMounted(() => {
  loadPayments();
});
</script>

<template>
  <div class="payments-view">
    <div class="mb-6 flex justify-between items-center">
      <div>
        <h2 class="text-2xl font-bold text-white">Payments</h2>
        <p class="text-gray-400">Manage pool payments</p>
      </div>
      <NSpace>
        <NSelect
          v-model:value="statusFilter"
          :options="statusOptions"
          style="width: 150px"
          @update:value="() => { currentPage = 1; loadPayments(); }"
        />
        <NPopconfirm
          @positive-click="handleTriggerPayment"
        >
          <template #trigger>
            <NButton
              type="primary"
              :loading="triggering"
            >
              <template #icon>
                <SendOutline />
              </template>
              Trigger Payment
            </NButton>
          </template>
          Manually trigger a payment run? This will process all pending payouts.
        </NPopconfirm>
      </NSpace>
    </div>

    <NSpin :show="loading">
      <NAlert v-if="error" type="error" class="mb-4">
        {{ error }}
      </NAlert>

      <NCard>
        <NDataTable
          :columns="columns"
          :data="payments"
          :pagination="{
            page: currentPage,
            pageSize: pageSize,
            pageCount: Math.ceil(total / pageSize),
            onChange: (page: number) => {
              currentPage = page;
              loadPayments();
            },
          }"
          :bordered="false"
        />
      </NCard>
    </NSpin>
  </div>
</template>

<style scoped>
.payments-view {
  max-width: 1400px;
  margin: 0 auto;
}

:deep(.n-card) {
  background-color: #1e293b !important;
  border-color: #334155 !important;
}
</style>
