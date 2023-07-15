<script lang="ts" setup>
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import type { Run } from "@/gql/graphql";
import { getCachedPercentage, isMostlyCached } from "@/state/session";
import { BoltIcon } from "@heroicons/vue/24/outline";

const now = useTimeFromNow();
const props = defineProps<{
  run: Run;
}>();
</script>
<template>
  <span v-if="isMostlyCached(run as any)" class="relative">
    <BoltIcon class="h-3 w-3 text-orange-500" />
    <span
      v-if="run.duration != null && run.cachedDuration != null"
      class="invisible absolute z-10 -ml-1 mt-1 w-36 rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 text-xs text-gray-700 group-hover/cache:visible"
    >
      Cached
      {{ now.getTimeFromNowString(run.cachedGeneratedAt) }} ago<br />
      <template v-if="getCachedPercentage(run) > 0">
        Saved {{ getCachedPercentage(run).toFixed() }}% (~{{
          formatDurationSeconds((run.cachedDuration - run.duration) * 1000)
        }})
      </template>
    </span>
  </span>
</template>
