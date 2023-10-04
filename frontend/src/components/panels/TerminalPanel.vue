<script lang="ts" setup>
import PanelHeader from "@/components/panels/PanelHeader.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext } from "@/state/bench";
import { computed } from "vue";

const props = defineProps<{ panel: PanelContext<LaunchRunPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }">
    <PanelHeader
      class="border-b border-orange-900 border-opacity-[12%] bg-gray-50"
      :editing="false"
      :thing="null"
      :actions="[]"
      :path="[]"
    />
    this da terminal
  </div>
</template>
