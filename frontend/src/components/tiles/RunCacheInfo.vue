<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
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
  return props.run.metadata?.[fieldKey];
}
const cachedDuration = computed(() => getMetadataValue("cached duration"));
const cachedGeneratedAt = computed(() => getMetadataValue("cached at"));

const isMostlyCached = computed(
  () => props.run.duration != null && cachedDuration.value != null && cachedDuration.value > props.run.duration * 0.8
);

const cachedPercentage = computed(() => {
  if (props.run.duration == null || cachedDuration.value == null) return 0;
  return (cachedDuration.value / props.run.duration) * 100;
});
</script>
<template>
  <span v-if="isMostlyCached" class="relative">
    <BoltIcon class="h-3 w-3 text-orange-500" />
    <span
      v-if="run.duration != null && cachedDuration != null"
      class="invisible absolute z-10 -ml-1 mt-1 w-36 rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 text-xs text-gray-700 group-hover/cache:visible"
    >
      Cached
      {{ now.getTimeFromNowString(cachedGeneratedAt) }} ago<br />
      <!-- TODO @Broken: run cached info saved % is incorrect? when did that happen? -->
      <!-- <template v-if="cachedPercentage > 0">
        Saved {{ cachedPercentage.toFixed() }}% (~{{ formatDuration((cachedDuration - run.duration) * 1000) }})
      </template> -->
    </span>
  </span>
</template>
