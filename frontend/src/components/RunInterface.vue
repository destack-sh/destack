<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type RunEditor, type StatementAction } from "@/state/editor";
import { symbolOf } from "@/state/runtime";
import { CommandLineIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref } from "vue";

const props = defineProps<{ editor: EditorContext<RunEditor>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const editor = computed(() => props.editor.editor.value);
const symbol = symbolOf(props.editor.editor.value.symbolId);

const symbolActions = computed(() => {
  const symbolActions: StatementAction[] = [];

  return symbolActions;
});
</script>
<template>
  <div class="relative flex flex-col overflow-x-hidden">
    <!-- Fixed inline header :EditorInlineHeader -->
    <div
      class="fixed z-10 flex flex-row items-center justify-between gap-1 border-b border-orange-900 border-opacity-[12%] bg-white px-1.5"
      :class="appearance.baseClass"
      :style="{ height: appearance.editorHeaderHeight + 'px', width: props.editor.size?.value?.width + 'px' }"
    >
      <!-- Main info -->
      <div class="flex flex-row items-center">
        <!-- editor actions -->
        <ActionPopover anchor="left" :thing="editor" :actions="props.editor.actions.value" class="">
          <CommandLineIcon class="mt-1 h-5 w-5 text-gray-500" />
        </ActionPopover>
        <!-- editor path -->
        <ActionPopover anchor="left" :thing="symbol" :actions="symbolActions" class="ml-1">
          <span class="text-gray-900">{{ symbol?.name }}</span>
        </ActionPopover>
        <span v-if="bench.debug" class="ml-2 bg-red-200 bg-opacity-50 text-gray-900">
          {{ bench.focusedEditorId == editor.id ? "(focused)" : "" }}
        </span>
      </div>
      <div class="flex flex-row gap-1">
        <!-- Opposite -->
      </div>
    </div>
    <!--  -->
  </div>
</template>
