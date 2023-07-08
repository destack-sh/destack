<script lang="ts" setup>
import ExecutionTraceback from "@/components/basic/ExecutionTraceback.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import StatementDeclaration from "@/components/statements/StatementDeclaration.vue";
import FunctionType from "@/components/statements/FunctionType.vue";
import InlineActions from "@/components/statements/StatementActions.vue";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { ExecutionStatus, type Execution } from "@/gql/graphql";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { EXECUTION_TERMINAL_STATES, useExecutions, isMostlyCached, getCachedPercentage } from "@/state/session";
import { TypeFlag, newExecutionId } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { useStatementContext } from "@/state/statement";
import {
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  NoSymbolIcon,
  PlayIcon,
  StopIcon,
  ArrowDownRightIcon,
  ArrowUpRightIcon,
  ArrowLongRightIcon,
  WindowIcon,
  TagIcon,
  EyeIcon,
  EyeSlashIcon,
} from "@heroicons/vue/24/outline";
import { BoltIcon } from "@heroicons/vue/20/solid";
import { nextTick, computed, ref, toRef, type Ref } from "vue";
import { DateTime } from "luxon";
import StatementTags from "@/components/statements/StatementTags.vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

const context = useStatementContext();
const editor = useEditorContext();
const ops = useOperations();
const notifications = useNotifications();

const code: Ref<string> = ref(context.statement.value.code ?? "");
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
const codeSync = context.syncCode(
  code,
  computed(() => monacoRef.value?.focused)
);

const now = useTimeFromNow(100);

// TODO @Performance: load inline code executions more sensibly
const bench = useBenchState();
const executions = useExecutions(
  {
    projectId: toRef(bench, "projectId"),
    projectVersionId: toRef(bench, "projectVersionId"),
    runnableIds: ref([context.statement.value.id]),
    includeAncestorVersions: ref(false),
  },
  { root: false, limit: 3, live: true }
);

const inputs = computed(() => context.fields.value.filter((f) => !(f.flags & TypeFlag.IsOutput)));
const outputs = computed(() => context.fields.value.filter((f) => f.flags & TypeFlag.IsOutput));
const numCodeLines = computed(() => code.value.split("\n").length);

const lastExecutionLocal: Ref<Execution | null> = ref(null); // triggered in this client session
const lastExecutionLocalId: Ref<string | null> = ref(null); // same but optimistic id
const lastExecution = computed(() => lastExecutionLocal.value ?? executions.executions.value[0]);
const lastExecutionId = computed(() => lastExecutionLocalId.value ?? lastExecution.value?.id ?? null);

const declarationRef: Ref<InstanceType<typeof StatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionType> | null> = ref(null);
const hasTypes = computed(() => context.fields.value.length > 0);
const addingTypes = ref(false);
const hideOutput = ref(true);
const truncateOutput = ref(true);
const preparingRun = ref(false);
const cancelled = ref(false);
const showTraceback = computed(
  () => lastExecution.value != null && lastExecution.value.status == ExecutionStatus.Failed && !hideOutput.value
);

const executionActive = computed(
  () =>
    preparingRun.value ||
    ops.state.hasInflightLike({ types: ["runtime.run"], keys: [context.statement.value.id] }) ||
    (lastExecution.value != null && !EXECUTION_TERMINAL_STATES.includes(lastExecution.value?.status))
);

function unfoldIfFolded() {
  if (props.folded) emit("toggleFold");
}

const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [
    {
      label: "Add tag",
      icon: TagIcon,
      action: () => {
        unfoldIfFolded();
        tagsRef.value?.open();
      },
      hideInline: true,
    },
    {
      label: "Add input",
      icon: ArrowDownRightIcon,
      action: () => {
        unfoldIfFolded();
        addingTypes.value = true;
        nextTick(() => typeRef.value?.createInput());
      },
      hideInline: true,
    },
    {
      label: "Add output",
      icon: ArrowUpRightIcon,
      action: () => {
        unfoldIfFolded();
        addingTypes.value = true;
        nextTick(() => typeRef.value?.createOutput());
      },
      hideInline: true,
    },
    {
      label: "Run",
      icon: PlayIcon,
      active: executionActive.value && !cancelled.value,
      disabled: hasTypes.value, // needs parameters
      action: async () => await run(),
    },
    {
      label: "Launch",
      icon: WindowIcon,
      action: () => {
        const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
        bench.openRun(context.statement.value, { group: nextGroup, focus: true });
      },
    },
  ];
  if (executionActive.value) {
    inlineActions.push({
      label: "Cancel",
      icon: StopIcon,
      action: async () => await cancel(),
    });
  } else {
    inlineActions.push({
      label: hideOutput.value ? "Show output" : "Hide output",
      disabled: lastExecution.value == null,
      icon: hideOutput.value ? EyeSlashIcon : EyeIcon,
      action: async () => {
        // TODO @Feature: clear execution for real?
        hideOutput.value = !hideOutput.value;
      },
      hideInline: props.folded,
    });
  }

  return inlineActions;
});
context.setCustomActions(extraActions);

async function run() {
  if (executionActive.value) {
    return; // already running
  }
  if (hasTypes.value) {
    const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
    bench.openRun(context.statement.value, { group: nextGroup, focus: true });
  } else {
    lastExecutionLocalId.value = newExecutionId();
    // optimistically set last execution local
    lastExecutionLocal.value = {
      id: lastExecutionLocalId.value,
      status: ExecutionStatus.Running,
      startedAt: now.now.value.toString(),
      createdAt: now.now.value.toString(),
      updatedAt: now.now.value.toString(),
      duration: null,
      cachedDuration: null,
      cachedGeneratedAt: null,
      inputs: null,
      outputs: null,
      error: null,
    } as Execution;
    cancelled.value = false;
    hideOutput.value = false;
    preparingRun.value = true; // for immediate feedback if flush takes more than few ms
    try {
      await codeSync.flushNow(); // flush any pending changes to the code (which is debounced)
    } finally {
      preparingRun.value = false;
    }
    // TODO @Robustness: ensure that executed code is exact same as in editor
    const ret = await ops.runtime.run(context.statement.value.id, lastExecutionLocalId.value);
    if (ret?.data?.run.__typename != "RunState" || !ret.data.run.success) {
      notifications.show({
        type: "run.fail",
        kind: "error",
        message: "Run failed",
        description: `Failed to run ${context.statement.value.name}: ${ret?.data?.run?.error ?? "rejected"}`,
      });
    }
    if (ret?.data?.run.__typename == "RunState") {
      lastExecutionLocal.value = (ret.data.run.execution as Execution) ?? null;
    }
  }
}

async function cancel() {
  cancelled.value = true;
  if (lastExecutionId.value == null) {
    return;
  }
  lastExecutionLocal.value = null;
  lastExecutionLocalId.value = null;
  await ops.runtime.cancel(lastExecutionId.value);
}

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first" || props.folded) {
      declarationRef.value?.focus();
    } else {
      monacoRef.value?.focus(true);
    }
  },
  blur: () => {
    declarationRef.value?.blur();
    typeRef.value?.blur();
    monacoRef.value?.blur();
  },
  run,
  loading: computed(() => false), // executions may be loading, but does not affect layout because output/state hidden by default
});
</script>
<template>
  <div class="flex flex-row justify-between">
    <!-- Declaration -->
    <div class="flex flex-row">
      <StatementDeclaration
        ref="declarationRef"
        class="inline-flex"
        @navigate-down="(typeRef?.focus ?? monacoRef?.focus ?? context.navigateDown)()"
      />
      <StatementTags ref="tagsRef" class="ml-1.5" />
    </div>
    <!-- Meta info & controls -->
    <div
      class="group/info flex flex-shrink-0 flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value || executionActive ? '' : 'opacity-0'"
    >
      <!-- Execution time -->
      <span
        v-if="!hasTypes && lastExecution != null"
        :class="[lastExecution?.status != ExecutionStatus.Failed || preparingRun ? 'text-gray-400' : 'text-red-600']"
      >
        {{
          formatDurationSeconds(
            (lastExecution?.duration ?? now.now.value.diff(DateTime.fromISO(lastExecution.startedAt)).as("seconds")) *
              1000
          )
        }}
      </span>
      <!-- Cache info -->
      <span
        v-if="!hasTypes && lastExecution != null && isMostlyCached(lastExecution as any)"
        class="relative mr-0.5 py-1"
      >
        <BoltIcon class="h-3 w-3 text-orange-500" />
        <span
          v-if="lastExecution.duration != null && lastExecution.cachedDuration != null"
          class="invisible absolute -right-10 z-10 -ml-1 mt-1 w-36 rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 text-xs text-gray-700 group-hover/info:visible"
        >
          Cached
          {{ now.getTimeFromNowString(lastExecution.cachedGeneratedAt) }} ago<br />
          <template v-if="getCachedPercentage(lastExecution) > 0">
            Saved {{ getCachedPercentage(lastExecution).toFixed() }}% (~{{
              formatDurationSeconds((lastExecution.cachedDuration - lastExecution.duration) * 1000)
            }})
          </template>
        </span>
      </span>
      <!-- Age -->
      <span
        v-if="!hasTypes"
        :class="[
          lastExecution?.status != ExecutionStatus.Failed || preparingRun ? 'text-gray-400' : 'text-red-600',
          lastExecution?.updatedAt ? 'opacity-100' : 'opacity-0',
        ]"
      >
        {{ now.getTimeFromNowString(lastExecution?.updatedAt) }}</span
      >
      <InlineActions :extraActions="extraActions" />
    </div>
  </div>
  <!-- Folded info -->
  <button
    v-if="folded"
    class="-mx-0.5 flex max-w-full flex-row gap-1.5 truncate rounded-sm px-0.5 text-gray-400 hover:bg-gray-100"
    @click="emit('toggleFold')"
  >
    <span>{{ numCodeLines }} {{ numCodeLines == 1 ? "line" : "lines" }}</span>
    •
    <span>Python</span>
    <template v-if="inputs.length + outputs.length > 0">•</template>
    <span v-for="input in inputs" :key="input.id">{{ input.name }}</span>
    <ArrowLongRightIcon v-if="outputs.length > 0" class="mt-0.5 h-4 w-4 text-gray-400" />
    <span v-for="output in outputs" :key="output.id">{{ output.name }}</span>
  </button>
  <!-- Type -->
  <FunctionType
    v-if="(hasTypes || addingTypes) && !folded"
    ref="typeRef"
    class="mb-2"
    @navigate-up="context.navigateUp"
    @navigate-down="monacoRef?.focus"
    @navigate-right="monacoRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Code -->
  <!-- TODO @UX: figure out nicer styling for code -->
  <MonacoEditor
    v-if="!folded"
    ref="monacoRef"
    :hide-line-numbers="false"
    :lineNumberOffset="0"
    :line-number-shift-px="context.xOffset.value - 20"
    v-model="code"
    @navigate-up="declarationRef?.focus"
    @navigate-down="context.navigateDown"
    @navigate-left="typeRef?.focus"
    @escape="context.escape"
    @enter="context.insertBelow"
    @execute="run"
    language="python"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class="-mx-1 mt-0.5 min-h-[32px] rounded-t-sm border border-orange-900 border-opacity-[15%] px-1 pb-1.5 pt-1 transition-colors duration-75"
    :class="[showTraceback ? '' : 'rounded-b-sm']"
  />
  <!-- Last output/error (if any) -->
  <ExecutionTraceback
    v-if="!hasTypes && showTraceback && !folded"
    class="relative -mx-1 mb-0.5 w-full rounded-b-sm border border-t-0 border-gray-200 px-3 py-1.5 font-mono transition duration-150"
    :class="[truncateOutput ? 'max-h-[300px] overflow-y-hidden' : '']"
    :key="lastExecution?.id"
    :name="context.statement.value?.name ?? 'run'"
    :execution="lastExecution"
  >
    <!-- If truncating, button overlay with fade gradient -->
    <button
      v-if="truncateOutput"
      class="group/truncate absolute bottom-0 left-0 flex h-12 w-full items-end justify-center bg-gradient-to-t from-white to-transparent pb-2"
      @click="truncateOutput = false"
    >
      <span class="p-0.5 text-gray-400 group-hover/truncate:animate-bounce group-hover/truncate:text-gray-800">
        <ChevronDoubleDownIcon class="h-4 w-4" />
      </span>
    </button>
    <button v-else class="group/truncate flex w-full flex-row justify-center pt-0.5" @click="truncateOutput = true">
      <ChevronDoubleUpIcon class="h-4 w-4 text-gray-400 group-hover/truncate:text-gray-800" />
    </button>
  </ExecutionTraceback>
</template>
