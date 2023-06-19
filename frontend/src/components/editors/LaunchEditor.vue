<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FixedInlineHeader from "@/components/editors/FixedInlineHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import ExecutionsTile from "@/components/tiles/ExecutionsTile.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type StatementAction, type LaunchEditor } from "@/state/bench";
import { FieldType } from "@/state/fragments";
import { newExecutionId, TypeFlag, useCurrentModule } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { unkey } from "@/state/type";
import { ArrowPathIcon, PlayIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, watchEffect } from "vue";

const props = defineProps<{ editor: EditorContext<LaunchEditor>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const notifications = useNotifications();
const ops = useOperations();
const editor = computed(() => props.editor.editor.value);
const editorSize = computed(() => props.editor.size.value);
const now = useTimeFromNow();

// state

const running = ref(false);
const module = useCurrentModule();
const statement = computed(() => module.statementOf(props.editor.editor.value.statementId));
const inputFields = computed(
  () =>
    statement.value?.fields
      ?.map((f) => useFragment(FieldType, f))
      .filter((t) => !(t.flags & TypeFlag.IsOutput))
      .map((t) => module.runtimeTypeOf(t)) ?? []
);
const outputFields = computed(
  () =>
    statement.value?.fields
      ?.map((f) => useFragment(FieldType, f))
      .filter((t) => t.flags & TypeFlag.IsOutput)
      .map((t) => module.runtimeTypeOf(t)) ?? []
);
const terminalActions = computed(() => {
  const actions: StatementAction[] = [];

  return actions;
});

// sync symbol type into editor
watchEffect(() => {
  if (statement.value?.type != null && statement.value.type != editor.value.statementType) {
    if (![StatementType.Code, StatementType.Task].includes(statement.value.type)) {
      throw new Error(`unexpected statement type ${statement.value.type}`);
    }
    editor.value.statementType = statement.value.type;
  }
});

// sync name/path into editor
const path = computed(() => {
  if (statement.value == null) return null;
  if (module.fileOf(statement.value) == null) return null;
  return module.fileOf(statement.value)?.name + ":" + statement.value?.name;
});
watch(path, () => {
  if (statement.value == null || module.idx.value == null) return;
  editor.value.updatePath(statement.value, module.idx.value);
});

async function run() {
  if (statement.value == null) return;
  editor.value.lastExecutionId = newExecutionId();
  const unkeyedArguments = unkey(inputFields.value, editor.value.arguments);
  running.value = true;
  const ret = await ops.runtime.run(
    editor.value.statementId,
    undefined,
    editor.value.lastExecutionId,
    unkeyedArguments
  );
  running.value = false;
  if (ret?.data?.run?.__typename == "RunState") {
    if (!ret?.data?.run?.success) {
      notifications.show({
        kind: "error",
        type: "run.failed",
        message: "Run failed",
        description: `${statement.value.name} could not be run.`,
      });
    } else {
      // notify on success if run took a bit
      if ((ret.data.run?.execution?.duration ?? 0) > 5) {
        notifications.show({
          kind: "success",
          type: "run.success",
          message: "Run completed",
          description: `${statement.value.name} finished after ${formatDurationSeconds(
            ret.data.run?.execution?.duration ?? 5
          )}.`,
        });
      }
      editor.value.lastOutput = ret.data.run.execution?.outputs;
      editor.value.lastExecutionTerminatedAt = ret.data.run.execution?.terminatedAt;
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
    targetWidth ?? editor.value.contentWidth,
    props.editor.size.value.width - 2 * editor.value.contentMarginX
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
    <!-- Fixed inline header -->
    <FixedInlineHeader
      class="border-b border-orange-900 border-opacity-[12%]"
      :editing="false"
      :thing="statement"
      :actions="terminalActions"
      :path="path"
    />
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
          <h1 class="text-3xl font-extrabold text-gray-900">{{ statement?.name ?? "" }}&nbsp;</h1>
          <h3 class="text-sm text-gray-700">{{ statement?.file?.path }}</h3>
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
      <ContainerTile
        label="Output"
        :sub-label="
          editor.lastExecutionTerminatedAt ? now.getTimeFromNowLongString(editor.lastExecutionTerminatedAt) : undefined
        "
        :style="{ ...baseTilePositionX }"
      >
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
      <ContainerTile v-if="statement != null" label="Runs" :style="{ ...baseTilePositionX }">
        <ExecutionsTile
          :project-id="bench.projectId"
          :project-version-id="bench.projectVersionId"
          include-ancestor-versions
          :runnable-id="editor?.statementId"
          :symbol-type="statement?.type"
          live
        />
      </ContainerTile>
    </div>
  </div>
</template>
