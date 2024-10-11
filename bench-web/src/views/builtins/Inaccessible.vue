<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { humanizeError } from "@/proto/services";
import { NodeType, type NodeReferenceData } from "@/proto/wire";
import type { Connection } from "@/system/connection";
import { DISCORD_URL } from "@/utils/globals";
import { getDurationFromNow, TimeUpdateInterval } from "@/utils/time";
import { DateTime } from "luxon";
import { computed } from "vue";

const props = defineProps<{ node?: NodeReferenceData; connection: Connection<any, any> }>();
const startedAt = DateTime.now();
const duration = computed(() => getDurationFromNow(startedAt, { updateInterval: TimeUpdateInterval.SECOND }));

const humanizedError = computed(() => {
  if (props.connection.lastError.value == null) return null;
  else return humanizeError(props.connection.lastError.value);
});
</script>
<template>
  <div class="flex flex-col justify-center text-center align-middle">
    <template v-if="connection.isConnected.value">
      <!-- Not found -->
      <span>
        <i class="fas fa-circle-exclamation mr-1.5 text-gray-500" />
        <span class="text-gray-600">{{ node != null ? toCamelName(NodeType, node.nodeType) : "Node" }} Not Found</span>
      </span>
      <!-- TODO :UX: help to restore node if not found (and is accessible, else help with policies) -->
    </template>
    <template v-else-if="!humanizedError || duration.as('seconds') < 5">
      <!-- Loading -->
      <!-- (delay appear to prevent flickering for very fast loads) -->
      <Transition
        enter-from-class="opacity-0"
        enter-active-class="transition-opacity duration-200"
        enter-to-class="opacity-100"
        appear
        mode="out-in"
      >
        <div v-if="duration.as('seconds') < 5">
          <!-- Regular spinny boi -->
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </div>
        <div v-else-if="duration.as('seconds') < 30" class="flex flex-col items-center">
          <!-- Something seems to be wrong -->
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
          <span class="mt-1.5 text-gray-700">Hold tight...</span>
        </div>
        <div v-else class="flex flex-col items-center">
          <!-- Couldn't connect -->
          <i class="fas fa-cloud-slash text-warning-600" />
          <span class="mt-1.5 text-gray-700">Unable to connect.</span>
          <p class="text-gray-400">
            (We'll try again. Check the <a :href="DISCORD_URL" class="mt-1 underline" target="_blank">Discord</a>.)
          </p>
        </div>
      </Transition>
    </template>
    <template v-else>
      <!-- Error -->
      <div class="w-fit self-center">
        <p>
          <i class="fas fa-circle-exclamation text-danger-600" />
          <span class="ml-1.5">
            <span class="font-semibold">{{ humanizedError.title }}</span>
          </span>
        </p>
        <p class="max-w-80 text-gray-900">{{ humanizedError.text }}</p>
        <p class="text-gray-400">
          (We'll try again. Check the <a :href="DISCORD_URL" class="mt-1 underline" target="_blank">Discord</a>.)
        </p>
      </div>
    </template>
  </div>
</template>
