<script lang="ts" setup>
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import type { Run } from "@/gql/graphql";
import { useCurrentModule } from "@/state/module";
import { BoltIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const now = useTimeFromNow();
const module = useCurrentModule();
const props = defineProps<{
  run: Run;
}>();

function getMetadataValue(name: string) {
  const fieldKey = module.runMetadataKey(name);
  if (fieldKey == null) return null;
  return props.run.value?.[fieldKey];
}
const cachedDuration = computed(() => getMetadataValue("cached duration"));
const cachedGeneratedAt = computed(() => getMetadataValue("cached at"));

const isMostlyCached = computed(
  () => props.run.duration != null && cachedDuration.value != null && cachedDuration.value > props.run.duration * 0.8
);
</script>
<template>
  <span v-if="isMostlyCached" class="group relative">
    <BoltIcon class="h-3 w-3 text-orange-500" />
    <span
      v-if="run.duration != null && cachedDuration != null"
      class="invisible absolute z-10 -ml-1 mt-1 w-fit whitespace-nowrap rounded-sm border border-orange-900/[12%] bg-white px-2 py-1 text-xs text-gray-700 group-hover:visible"
    >
      Cached
      {{ now.getTimeFromNowString(cachedGeneratedAt) }} ago<br />
    </span>
  </span>
</template>
