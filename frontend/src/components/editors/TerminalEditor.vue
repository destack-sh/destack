<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type TerminalEditor, type StatementAction } from "@/state/bench";
import { newExecutionId, symbolOf, TypeFlag } from "@/state/runtime";
import { CommandLineIcon } from "@heroicons/vue/24/outline";
import { PlayIcon, ArrowPathIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watchEffect } from "vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import ExecutionsTile from "@/components/tiles/ExecutionsTile.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useOperations } from "@/state/operations";
import { SymbolType } from "@/gql/graphql";
import { unkey } from "@/state/type";
import { useNotifications } from "@/state/notifications";

const props = defineProps<{ editor: EditorContext<TerminalEditor>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const notifications = useNotifications();
const ops = useOperations();
const editor = computed(() => props.editor.editor.value);
const editorSize = computed(() => props.editor.size.value);

// state

const running = ref(false);
const symbol = computed(() => symbolOf(props.editor.editor.value.symbolId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => symbol.value?.typeNodes?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);
const symbolActions = computed(() => {
  const symbolActions: StatementAction[] = [];

  return symbolActions;
});

// sync symbol type into editor
watchEffect(() => {
  if (symbol.value?.symbolType != null && symbol.value.symbolType != editor.value.symbolType) {
    if (![SymbolType.Code, SymbolType.Task].includes(symbol.value.symbolType)) {
      throw new Error(`unexpected symbol type ${symbol.value.symbolType}`);
    }
    editor.value.symbolType = symbol.value.symbolType;
  }
});

async function run() {
  if (symbol.value == null) return;
  editor.value.lastExecutionId = newExecutionId();
  const unkeyedArguments = unkey(inputFields.value, editor.value.arguments);
  running.value = true;
  const ret = await ops.runtime.run(editor.value.symbolId, undefined, editor.value.lastExecutionId, unkeyedArguments);
  running.value = false;
  if (ret?.data?.run?.__typename == "RunState") {
    if (!ret?.data?.run?.success) {
      notifications.show({
        kind: "error",
        type: "run.failed",
        message: "Run failed",
        description: `${symbol.value.name} could not be run.`,
      });
    } else {
      // notify on success if run took a bit
      if ((ret.data.run?.execution?.duration ?? 0) > 5) {
        notifications.show({
          kind: "success",
          type: "run.success",
          message: "Run completed",
          description: `${symbol.value.name} completed`,
        });
      }
      editor.value.lastOutput = ret.data.run.execution?.outputs;
    }
  }
}

// tiling

// TODO @UX: auto scale grid step based on available width
//  This is just a crude placeholder to experiment.
const showDots = ref(false);
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

function getTilePositionX(targetWidth?: number) {
  const tileWidth = getTileWidth(targetWidth);
  const tileOffsetX = getTileOffsetX(targetWidth);
  return {
    width: tileWidth + "px",
    marginLeft: tileOffsetX + "px",
    marginRight: tileOffsetX + "px",
  };
}

const baseTilePositionX = computed(() => getTilePositionX());

// navigation

function focus() {
  // nothing to do yet
}

function blur() {
  // nothing to do yet
}

defineExpose({
  focus,
  blur,
});
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
      class="relative flex h-full w-full flex-col gap-6"
      :class="appearance.baseClass"
      :style="{
        marginTop: appearance.editorHeaderHeight + 'px',
        paddingTop: gridStepY + 'px',
        paddingBottom: gridStepY + 'px',
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
            <!-- color is gray-300 -->
            <circle fill="#d4d4d8" :cx="dotSize" :cy="dotSize" :r="dotSize" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#dots)" />
      </svg>
      <!-- Header -->
      <div class="z-[1] flex flex-row items-baseline justify-between p-2" :style="baseTilePositionX">
        <!-- Title & source -->
        <div class="flex flex-col">
          <h1 class="text-3xl font-extrabold text-gray-900">{{ symbol?.name ?? "" }}&nbsp;</h1>
          <h3 class="text-sm text-gray-700">{{ symbol?.file?.path }}</h3>
        </div>
        <!-- Run button -->
        <div
          class=""
          :style="{
            height: gridStepY * 2 + 'px',
            width: getTileWidth(gridStepX * 3) + 'px',
          }"
        >
          <button
            class="flex h-full w-full flex-row items-center justify-center gap-1 rounded-sm bg-orange-500 text-white hover:bg-orange-400 focus:bg-orange-400"
            @click="run"
            @keydown.enter.exact.prevent="run"
            :disabled="running"
          >
            Run
            <FadeTransition mode="out-in">
              <component
                :is="running ? ArrowPathIcon : PlayIcon"
                class="h-4 w-4"
                :class="[running ? 'animate-spin' : '']"
              />
            </FadeTransition>
          </button>
        </div>
      </div>
      <!-- Input -->
      <ContainerTile label="Input" :style="{ ...baseTilePositionX }">
        <StructTile v-if="inputFields.length > 0" v-model="editor.arguments" :fields="inputFields" class="" />
        <div v-else class="flex h-full w-full flex-col items-center justify-center">
          <span class="text-sm text-gray-400">No input</span>
        </div>
      </ContainerTile>
      <!-- Output -->
      <ContainerTile label="Output" :style="{ ...baseTilePositionX }">
        <StructTile
          v-if="editor.lastOutput"
          :model-value="editor.lastOutput"
          :fields="outputFields"
          readonly
          class=""
        />
        <div v-else class="flex h-full w-full flex-col items-center justify-center">
          <span class="text-sm text-gray-400">No output</span>
        </div>
      </ContainerTile>
      <!-- Executions -->
      <ContainerTile v-if="symbol != null" label="Runs" :style="{ ...baseTilePositionX }">
        <ExecutionsTile
          :project-id="bench.currentProjectId"
          :project-version-id="bench.currentProjectVersionId"
          include-ancestor-versions
          :runnable-id="editor?.symbolId"
          :symbol-type="symbol?.symbolType"
          live
        />
      </ContainerTile>
    </div>
  </div>
</template>
