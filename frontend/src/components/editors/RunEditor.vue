<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type RunEditor, type StatementAction } from "@/state/editor";
import { symbolOf } from "@/state/runtime";
import { CommandLineIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";

const props = defineProps<{ editor: EditorContext<RunEditor>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const editor = computed(() => props.editor.editor.value);
const editorSize = computed(() => props.editor.size.value);
const symbol = symbolOf(props.editor.editor.value.symbolId);

const symbolActions = computed(() => {
  const symbolActions: StatementAction[] = [];

  return symbolActions;
});

const showDots = ref(true);
const dotSize = ref(1);
const gridStepX = ref(36); // p-9
const gridStepY = ref(18); // p-4.5

function getTileWidth(targetWidth?: number) {
  return Math.min(
    targetWidth ?? appearance.contentWidth,
    props.editor.size.value.width - 2 * appearance.contentMarginX
  );
}

function getTileOffsetX(targetWidth?: number) {
  return (props.editor.size.value.width - getTileWidth(targetWidth)) / 2;
}

const defaultTileWidth = computed(() => getTileWidth());
const defaultTileOffsetX = computed(() => getTileOffsetX());
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: editorSize.height + 'px' }">
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
          <!-- (this is deliberately 5x5 instead of 4x4 since 4x4 looks tiny compared to code bracket in file editor) -->
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
    <!-- Tiles -->
    <div
      class="relative flex h-full w-full flex-col gap-5"
      :style="{
        marginTop: appearance.editorHeaderHeight + 'px',
        minHeight: editorSize.height - appearance.editorHeaderHeight + 'px',
      }"
    >
      <!-- Dot background -->
      <svg
        v-if="showDots"
        xmlns="http://www.w3.org/2000/svg"
        class="absolute h-full w-full"
        :style="{
          top: gridStepY + 'px',
          left: gridStepX + 'px',
          maxWidth: 'calc(100% - ' + gridStepX + 'px)',
          maxHeight: 'calc(100% - ' + gridStepY + 'px)',
        }"
      >
        <defs>
          <pattern id="dots" patternUnits="userSpaceOnUse" :width="gridStepX" :height="gridStepY">
            <circle fill="#e4e4e7" :cx="dotSize" :cy="dotSize" :r="dotSize" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#dots)" />
      </svg>
      <!-- Foreground -->
      <!-- Header -->
      <!-- Input -->
      <div
        class="z-[1] mt-10 rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-2 shadow-sm"
        :style="{
          height: '250px',
          width: defaultTileWidth + 'px',
          marginLeft: defaultTileOffsetX + 'px',
          marginRight: defaultTileOffsetX + 'px',
        }"
      >
        inputs to {{ symbol?.name }}
      </div>
      <!-- Run button -->
      <div
        class="z-[1] rounded-sm border border-orange-900 border-opacity-[12%] bg-white shadow-sm"
        :style="{
          height: gridStepY * 2 + 'px',
          width: getTileWidth(gridStepX * 4) + 'px',
          marginLeft: getTileOffsetX(gridStepX * 4) + 'px',
          marginRight: getTileOffsetX(gridStepX * 4) + 'px',
        }"
      >
        <button class="h-full w-full bg-orange-500 text-white">Run</button>
      </div>
      <!-- Output -->
      <!-- Executions -->
    </div>
  </div>
</template>
