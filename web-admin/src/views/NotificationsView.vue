<script setup lang="ts">
import { ref, onMounted } from 'vue';
import {
  NCard,
  NSwitch,
  NSpin,
  NAlert,
  NDivider,
} from 'naive-ui';
import { fetchNotifications, updateNotification } from '../api';
import type { NotificationConfig } from '../types';

const loading = ref(true);
const error = ref<string | null>(null);
const notifications = ref<NotificationConfig[]>([]);

async function loadNotifications() {
  try {
    loading.value = true;
    error.value = null;
    notifications.value = await fetchNotifications();
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load notifications';
  } finally {
    loading.value = false;
  }
}

async function handleToggleEnabled(config: NotificationConfig) {
  try {
    await updateNotification(config.id, { enabled: !config.enabled });
    await loadNotifications();
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to update notification';
  }
}

onMounted(() => {
  loadNotifications();
});
</script>

<template>
  <div class="notifications-view">
    <div class="mb-6">
      <h2 class="text-2xl font-bold text-white">Notifications</h2>
      <p class="text-gray-400">Configure notification channels</p>
    </div>

    <NSpin :show="loading">
      <NAlert v-if="error" type="error" class="mb-4">
        {{ error }}
      </NAlert>

      <div v-for="config in notifications" :key="config.id" class="mb-4">
        <NCard>
          <div class="flex justify-between items-center">
            <div>
              <h3 class="text-lg font-semibold text-white capitalize">
                {{ config.type }} Notifications
              </h3>
              <p class="text-gray-400 text-sm">
                {{ config.enabled ? 'Enabled' : 'Disabled' }}
              </p>
            </div>
            <NSwitch
              :value="config.enabled"
              @update:value="() => handleToggleEnabled(config)"
            />
          </div>
        </NCard>
      </div>

      <NDivider />

      <NCard>
        <h3 class="text-lg font-semibold text-white mb-4">Add Notification Channel</h3>
        <p class="text-gray-400">
          Notification configuration is done via config file or environment variables.
          Available channels: Telegram, Email
        </p>
      </NCard>
    </NSpin>
  </div>
</template>

<style scoped>
.notifications-view {
  max-width: 800px;
  margin: 0 auto;
}

:deep(.n-card) {
  background-color: #1e293b !important;
  border-color: #334155 !important;
}
</style>
