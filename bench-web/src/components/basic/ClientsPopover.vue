<script lang="ts" setup>
import UserAvatar from "@/components/basic/UserAvatar.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useCurrentClients } from "@/state/client";
import { useCurrentModule } from "@/state/module";
import { computed } from "vue";

const props = defineProps<{
  size: "large" | "small";
  fileId?: string;
  statementId?: string;
  first?: number;
}>();
const first = computed(() => props.first ?? 4);
const module = useCurrentModule();

const { activeClientsWithoutSelf: clients } = useCurrentClients();
const filteredClients = computed(() =>
  clients.value.filter(
    (c) =>
      props.fileId == null ||
      (c.fileId == props.fileId && props.statementId == null) ||
      c.statementId == props.statementId
  )
);

const now = useTimeFromNow();
</script>
<template>
  <div class="flex flex-row items-baseline gap-1">
    <!-- :ProfilePreview -->
    <div v-for="client in filteredClients.slice(0, first)" :key="client.id" class="group/popover relative">
      <UserAvatar
        :user="client.user"
        :clientId="client.id"
        class="text-gray-900"
        :class="props.size == 'large' ? 'h-7 w-7' : 'h-6 w-6'"
      />
      <!-- Profile info popover -->
      <div
        class="invisible absolute right-0 z-30 mt-3 w-60 origin-bottom-right bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-orange-900 ring-opacity-40 group-hover/popover:visible"
      >
        <div class="flex flex-row items-baseline justify-between">
          <span class="text-gray-900">
            {{ client.user.username }}
          </span>
        </div>
        <p class="text-xs text-gray-500">{{ client.user.name }}</p>
        <p class="mt-2 flex flex-col text-gray-900">
          <span v-if="module.idx.value?.filesById[client.fileId ?? '']?.name" class="ml-0">
            {{ module.idx.value?.filesById[client.fileId ?? ""]?.name }}</span
          >
          <span v-if="module.statementOf(client.statementId ?? '')" class="ml-0">
            > {{ module.statementOf(client.statementId ?? "")?.name }}
          </span>
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
