<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useBenchState, usePanelContext } from "@/state/bench";
import { getRunStatusColor, useCurrentSessions } from "@/state/session";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const now = useTimeFromNow(1000);
const sessions = useCurrentSessions();
const bench = useBenchState();
const panel = usePanelContext();

const { currentRun, currentRunActive } = sessions.currentRunOf(props.statement);
</script>
<template>
  <div
    class="group/info flex flex-shrink-0 flex-row items-center gap-1 rounded-sm px-0.5 transition duration-150 hover:bg-orange-100 group-hover/statement:opacity-100"
    :class="focused || currentRunActive ? '' : 'opacity-0'"
  >
    <BusySpinnerIcon v-if="currentRun != null && currentRun.startedAt == null" class="h-4 w-4 animate-spin" />
    <span
      class="rounded-sm underline-offset-2 hover:cursor-pointer"
      @click="bench.openViewRun(currentRun, { group: panel.panel.value.group, focus: true, opposite: true })"
    >
      <!-- Duration -->
      <span v-if="currentRun != null" :class="[getRunStatusColor(currentRun.status, { gray: 'text-gray-400' })]">
        {{ sessions.getDurationFormatted(currentRun) }}
      </span>
      <!-- Age -->
      <span
        class="ml-1"
        :class="[
          getRunStatusColor(currentRun?.status, { gray: 'text-gray-400' }),
          currentRun?.updatedAt ? 'opacity-100' : 'opacity-0',
        ]"
      >
        {{ now.getTimeFromNowString(currentRun?.updatedAt) }}</span
      >
    </span>
    <!-- Cache info -->
    <RunCacheInfo v-if="currentRun != null" :run="currentRun" class="relative mr-0.5 py-1" />
  </div>
</template>
