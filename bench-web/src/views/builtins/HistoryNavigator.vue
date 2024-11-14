<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { HISTORY_STATE_KEY, HistoryState } from "@/ui/view";
import { computed, inject } from "vue";

const props = defineProps<{
  self: TypedNodeReferenceData<NodeType.VIEW> | undefined;
}>();
const history = inject<HistoryState | null>(HISTORY_STATE_KEY);
const isActive = computed(() => history?.focusedView?.value?.id == props.self?.id);

defineExpose({
  isActive,
});
</script>
<template>
  <div v-if="isActive" class="flex flex-row gap-x-1 px-1">
    <button
      class="rounded px-0.5 py-0.5 text-gray-400 enabled:text-gray-700 enabled:hover:bg-gray-100"
      :disabled="!history?.canGoBackward.value"
      @click="history?.goBackward()"
    >
      <i class="fas fa-arrow-left" />
    </button>
    <button
      class="rounded px-0.5 py-0.5 text-gray-400 enabled:text-gray-700 enabled:hover:bg-gray-100"
      :disabled="!history?.canGoForward.value"
      @click="history?.goForward()"
    >
      <i class="fas fa-arrow-right" />
    </button>
  </div>
  <div v-else>
    <!-- Inactive History -->
  </div>
</template>
