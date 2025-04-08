<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { CommandBuiltinId } from "@/ui/command";
import { HISTORY_STATE_KEY, HistoryState } from "@/ui/view";
import { computed, inject } from "vue";

const props = defineProps<{
  self: TypedNodeReferenceData<NodeType.VIEW> | undefined;
}>();
const history = inject<HistoryState | null>(HISTORY_STATE_KEY, null);
const isActive = computed(() => history?.focusedView?.value?.id == props.self?.id);

defineExpose({
  isActive,
});
</script>
<template>
  <div v-if="isActive" class="flex flex-row gap-x-1 px-1">
    <button
      v-tooltip="{
        group: 'history',
        placement: 'bottom',
        title: 'Go back',
        commands: ['view.history.goBackward'] as CommandBuiltinId[],
      }"
      class="rounded px-0.5 py-0.5 text-gray-400 enabled:text-gray-700 enabled:hover:bg-gray-100"
      :disabled="!history?.canGoBackward.value"
      @click="history?.go(-1)"
    >
      <i class="fas fa-arrow-left" />
    </button>
    <button
      v-tooltip="{
        group: 'history',
        placement: 'bottom',
        title: 'Go forward',
        commands: ['view.history.goForward'] as CommandBuiltinId[],
      }"
      class="rounded px-0.5 py-0.5 text-gray-400 enabled:text-gray-700 enabled:hover:bg-gray-100"
      :disabled="!history?.canGoForward.value"
      @click="history?.go(1)"
    >
      <i class="fas fa-arrow-right" />
    </button>
  </div>
  <div v-else>
    <!-- Inactive History -->
  </div>
</template>
