<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { getClientColor, useCurrentClients } from "@/state/client";

const props = defineProps<{ size: "large" | "medium" | "small" }>();

const { activeClientsWithoutSelf: clients } = useCurrentClients();

const now = useTimeFromNow();
</script>
<template>
  <div class="flex flex-row gap-1">
    <!-- :ProfilePreview -->
    <div
      v-for="client in clients.slice(0, 4)"
      :key="client.id"
      class="group relative border border-orange-900 border-opacity-[15%] bg-orange-100 px-2.5 py-1"
      :style="{
        backgroundColor: getClientColor(client.id),
      }"
    >
      <span class="text-sm font-bold text-gray-900">
        {{ client.user.username.slice(0, 2).toLocaleUpperCase() }}
      </span>
      <!-- Info popover -->
      <div
        class="invisible absolute right-0 z-10 mt-3 w-72 origin-bottom-right bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-orange-900 ring-opacity-40 group-hover:visible"
      >
        <div class="flex flex-row items-baseline justify-between">
          <span class="font-bold text-gray-900">
            {{ client.user.username }}
          </span>
        </div>
        <p class="text-xs text-gray-500">{{ client.user.email }}</p>
        <p class="mt-2 flex flex-col font-bold text-gray-900">
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
  </div>
</template>
