<script setup lang="ts">
import { NLayout, NLayoutSider, NLayoutContent, NMenu, NIcon, NMessageProvider } from 'naive-ui';
import { RouterView, useRouter, useRoute } from 'vue-router';
import { h, ref, computed } from 'vue';
import type { MenuOption } from 'naive-ui';

import {
  StatsChartOutline,
  PeopleOutline,
  CardOutline,
  NotificationsOutline,
  SettingsOutline,
} from '@vicons/ionicons5';

const router = useRouter();
const route = useRoute();
const collapsed = ref(false);

const menuOptions: MenuOption[] = [
  {
    label: 'Dashboard',
    key: 'dashboard',
    icon: () => h(NIcon, null, { default: () => h(StatsChartOutline) }),
  },
  {
    label: 'Miners',
    key: 'miners',
    icon: () => h(NIcon, null, { default: () => h(PeopleOutline) }),
  },
  {
    label: 'Payments',
    key: 'payments',
    icon: () => h(NIcon, null, { default: () => h(CardOutline) }),
  },
  {
    label: 'Notifications',
    key: 'notifications',
    icon: () => h(NIcon, null, { default: () => h(NotificationsOutline) }),
  },
  {
    label: 'Settings',
    key: 'settings',
    icon: () => h(NIcon, null, { default: () => h(SettingsOutline) }),
  },
];

const activeKey = computed(() => route.name as string);

function handleMenuSelect(key: string) {
  router.push({ name: key });
}
</script>

<template>
  <NMessageProvider>
    <NLayout has-sider class="h-screen">
      <NLayoutSider
        bordered
        collapse-mode="width"
        :collapsed-width="64"
        :width="240"
        :collapsed="collapsed"
        show-trigger
        @collapse="collapsed = true"
        @expand="collapsed = false"
      >
        <div class="h-16 flex items-center justify-center border-b border-gray-700">
          <h1 v-if="!collapsed" class="text-xl font-bold text-ocean-primary">DMPool Admin</h1>
          <span v-else class="text-2xl font-bold text-ocean-primary">DM</span>
        </div>
        <NMenu
          :collapsed="collapsed"
          :collapsed-width="64"
          :collapsed-icon-size="22"
          :options="menuOptions"
          :value="activeKey"
          @update:value="handleMenuSelect"
        />
      </NLayoutSider>
      <NLayout>
        <NLayoutContent class="p-6">
          <RouterView />
        </NLayoutContent>
      </NLayout>
    </NLayout>
  </NMessageProvider>
</template>

<style>
@import './assets/styles.css';
</style>
