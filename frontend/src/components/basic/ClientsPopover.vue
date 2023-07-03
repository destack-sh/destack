<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { getClientColor, useCurrentClients } from "@/state/client";
import { computed } from "vue";

const props = defineProps<{
  size: "large" | "medium" | "small";
  fileId?: string;
  statementId?: string;
  first?: number;
}>();
const first = computed(() => props.first ?? 4);

const { activeClientsWithoutSelf: clients } = useCurrentClients();
const filteredClients = computed(() =>
  clients.value.filter(
    (c) =>
      props.fileId == null ||
      (c.file?.id == props.fileId && props.statementId == null) ||
      c.statement?.id == props.statementId
  )
);

const now = useTimeFromNow();
</script>
<template>
  <div class="flex flex-row items-baseline gap-1">
    <!-- :ProfilePreview -->
    <div
      v-for="client in filteredClients.slice(0, first)"
      :key="client.id"
      class="group relative rounded-sm border border-orange-900 border-opacity-[15%] bg-orange-100"
      :class="{
        'px-2.5 py-1': props.size === 'large',
        'px-1.5 py-0.5': props.size === 'medium',
        'px-1 py-0.5': props.size === 'small',
      }"
      :style="{
        backgroundColor: getClientColor(client.id),
      }"
    >
      <span class="text-sm text-gray-900">
        {{ client.user.username.slice(0, 2).toLocaleUpperCase() }}
      </span>
      <!-- Profile info popover -->
      <div
        class="invisible absolute right-0 z-30 mt-3 w-60 origin-bottom-right bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-orange-900 ring-opacity-40 group-hover:visible"
      >
        <div class="flex flex-row items-baseline justify-between">
          <span class="text-gray-900">
            {{ client.user.username }}
          </span>
        </div>
        <p class="text-xs text-gray-500">{{ client.user.name }}</p>
        <!-- TODO @Broken @UX: show clients file/statement again (name is no longer part of client data) -->
        <p class="mt-2 flex flex-col text-gray-900">
          <span
            >{{ client.project?.name }}
            <span v-if="client.file?.name" class="ml-0"> / {{ client.file?.name }}</span>
          </span>
          <span v-if="client.statement?.name" class="ml-0"> > {{ client.statement?.name }} </span>
        </p>
        <p class="mt-2 flex flex-row justify-between text-xs text-gray-500">
          {{ now.getTimeFromNowLongString(client.lastSeenAt) }}
          <span>{{ client.browserName }} - {{ client.deviceName }}</span>
        </p>
      </div>
    </div>
    <!-- +x -->
    <div v-if="filteredClients.length > first" class="text-sm text-gray-900">+{{ filteredClients.length - first }}</div>
  </div>
</template>
