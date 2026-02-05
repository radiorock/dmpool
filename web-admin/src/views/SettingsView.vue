<script setup lang="ts">
import { ref, onMounted } from 'vue';
import {
  NCard,
  NSpin,
  NAlert,
  NFormItem,
  NInput,
  NButton,
  NSpace,
  useDialog,
} from 'naive-ui';
import { fetchConfigs, updateConfig } from '../api';
import type { SystemConfig } from '../types';

const dialog = useDialog();
const loading = ref(true);
const saving = ref(false);
const error = ref<string | null>(null);
const configs = ref<SystemConfig[]>([]);
const editingConfigs = ref<Record<string, string>>({});

async function loadConfigs() {
  try {
    loading.value = true;
    error.value = null;
    configs.value = await fetchConfigs();
    // Initialize editing values
    configs.value.forEach((config) => {
      editingConfigs.value[config.key] = config.value;
    });
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load configs';
  } finally {
    loading.value = false;
  }
}

async function handleSaveConfig(key: string) {
  try {
    saving.value = true;
    const value = editingConfigs.value[key];
    if (value === undefined) {
      throw new Error('Configuration value is undefined');
    }
    await updateConfig(key, value);
    dialog.success({
      title: 'Success',
      content: 'Configuration updated',
    });
    await loadConfigs();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to update config',
    });
  } finally {
    saving.value = false;
  }
}

async function handleSaveAll() {
  try {
    saving.value = true;
    for (const [key, value] of Object.entries(editingConfigs.value)) {
      await updateConfig(key, value);
    }
    dialog.success({
      title: 'Success',
      content: 'All configurations updated',
    });
    await loadConfigs();
  } catch (err) {
    dialog.error({
      title: 'Error',
      content: err instanceof Error ? err.message : 'Failed to update configs',
    });
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  loadConfigs();
});
</script>

<template>
  <div class="settings-view">
    <div class="mb-6 flex justify-between items-center">
      <div>
        <h2 class="text-2xl font-bold text-white">Settings</h2>
        <p class="text-gray-400">System configuration</p>
      </div>
      <NButton
        type="primary"
        :loading="saving"
        @click="handleSaveAll"
      >
        Save All
      </NButton>
    </div>

    <NSpin :show="loading">
      <NAlert v-if="error" type="error" class="mb-4">
        {{ error }}
      </NAlert>

      <NCard>
        <div v-for="config in configs" :key="config.key" class="mb-4">
          <NFormItem :label="config.key">
            <template #label>
              <div>
                <span class="text-white">{{ config.key }}</span>
                <p class="text-gray-400 text-sm">{{ config.description }}</p>
              </div>
            </template>
            <NSpace vertical style="width: 100%">
              <NInput
                v-model:value="editingConfigs[config.key]"
                type="textarea"
                :autosize="{ minRows: 1, maxRows: 5 }"
              />
              <NButton
                size="small"
                :loading="saving"
                @click="handleSaveConfig(config.key)"
              >
                Save
              </NButton>
            </NSpace>
          </NFormItem>
        </div>
      </NCard>
    </NSpin>
  </div>
</template>

<style scoped>
.settings-view {
  max-width: 800px;
  margin: 0 auto;
}

:deep(.n-card) {
  background-color: #1e293b !important;
  border-color: #334155 !important;
}

:deep(.n-form-item-label) {
  color: #94a3b8 !important;
}
</style>
