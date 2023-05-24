<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import FunctionTypeCell from "@/components/cells/FunctionTypeCell.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import { useTimeFromNow } from "@/composables/useNow";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/editor";
import { useExecutions } from "@/state/executions";
import { computed, toRef, ref, type Ref } from "vue";
import { newExecutionId, useSymbolOps } from "@/state/runtime";
import { ExecutionStatus, type Execution } from "@/gql/graphql";
import InlineActions from "@/components/basic/InlineActions.vue";
import {
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  NoSymbolIcon,
  PlayIcon,
  StopIcon,
} from "@heroicons/vue/24/outline";
import { EXECUTION_TERMINAL_STATES } from "@/state/executions";
import { formatDurationSeconds } from "@/composables/useNow";
import { useOperations } from "@/state/operations";

const context = useStatementContext();
const editor = useEditorContext();

const ops = useOperations();
const symbolOps = useSymbolOps();
const code: Ref<string> = ref(context.statement.value.code ?? "");
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
const codeSync = context.syncCode(
  code,
  computed(() => monacoRef.value?.focused)
);

const now = useTimeFromNow();

// TODO @Performance: load inline code executions more sensibly
const bench = useBenchState();
const executions = useExecutions(
  {
    projectId: toRef(bench, "currentProjectId"),
    projectVersionId: toRef(bench, "currentProjectVersionId"),
    codeIds: ref([context.statement.value.id]),
    buildIds: ref(null),
    includeAncestorVersions: ref(false),
    taskIds: ref(null),
  },
  { root: true, limit: 3, live: true }
);

const lastExecutionLocal: Ref<Execution | null> = ref(null); // triggered in this client session
const lastExecutionLocalId: Ref<string | null> = ref(null); // same but optimistic id
const lastExecution = computed(() => lastExecutionLocal.value ?? executions.executions.value[0]);
const lastExecutionId = computed(() => lastExecutionLocalId.value ?? lastExecution.value?.id ?? null);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);
const hasTypes = computed(() => context.typeNodes.value.length > 0);
const addingTypes = ref(false);
const hideOutput = ref(false);
const truncateOutput = ref(true);
const preparingRun = ref(false);

const executionActive = computed(
  () =>
    preparingRun.value ||
    ops.state.hasInflightLike({ types: ["runtime.run"], keys: [context.statement.value.id] }) ||
    (lastExecution.value != null && !EXECUTION_TERMINAL_STATES.includes(lastExecution.value?.status))
);
const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [
    {
      label: "Run",
      icon: PlayIcon,
      active: executionActive.value,
      action: async () => await run(),
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
      label: "Clear",
      icon: NoSymbolIcon,
      action: async () => {
        // TODO @Feature: clear execution for real?
        hideOutput.value = !hideOutput.value;
      },
    });
  }

  return inlineActions;
});

async function run() {
  if (executionActive.value) {
    return; // already running
  }
  if (hasTypes.value) {
    const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
    bench.openRun(context.statement.value, { group: nextGroup, focus: true });
  } else {
    lastExecutionLocalId.value = newExecutionId();
    hideOutput.value = false;
    preparingRun.value = true; // for immediate feedback if flush takes more than few ms
    try {
      await codeSync.flushNow(); // flush any pending changes to the code (debounced)
    } finally {
      preparingRun.value = false;
    }
    // TODO @Robustness: ensure that executed code is exact same as in editor
    const ret = await symbolOps.run(context.statement.value, lastExecutionLocalId.value);
    if (ret?.data?.run.__typename == "RunState") {
      lastExecutionLocal.value = (ret.data.run.execution as Execution) ?? null;
    }
  }
}

async function cancel() {
  if (lastExecutionId.value == null) {
    return;
  }
  lastExecutionLocal.value = null;
  lastExecutionLocalId.value = null;
  await symbolOps.cancel(lastExecutionId.value);
}

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
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
});
</script>
<template>
  <div class="flex flex-row justify-between">
    <!-- Declaration -->
    <div>
      <DeclarationCell
        ref="declarationRef"
        class="inline-flex"
        @navigate-down="monacoRef?.focus"
        @navigate-right="typeRef?.focus"
      />
      <!-- Inline type -->
      <button
        v-if="!context.readonly.value && context.typeNodes.value.length == 0"
        ref="typeRef"
        class="z-10 ml-2 w-fit rounded-sm px-0.5 text-sm hover:bg-orange-100 hover:text-gray-700"
        :class="context.focused.value ? 'text-gray-400' : 'text-gray-300'"
        @click="addingTypes = !addingTypes"
      >
        {{ addingTypes ? "-arguments" : "+arguments" }}
      </button>
    </div>
    <!-- Meta info & controls -->
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value || executionActive ? '' : 'opacity-0'"
    >
      <span
        :class="[
          lastExecution?.status != ExecutionStatus.Failed || preparingRun ? 'text-gray-400' : 'text-red-600',
          EXECUTION_TERMINAL_STATES.includes(lastExecution?.status) ? 'opacity-100' : 'opacity-0',
        ]"
      >
        {{ formatDurationSeconds((lastExecution?.duration ?? 0) * 1000) }}
      </span>
      <span
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
  <FunctionTypeCell
    v-if="hasTypes || addingTypes"
    ref="typeRef"
    class=""
    @navigate-up="context.navigateUp"
    @navigate-down="monacoRef?.focus"
    @navigate-right="monacoRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Code -->
  <!-- TODO @UX: figure out nicer styling for code -->
  <MonacoEditor
    ref="monacoRef"
    hide-line-numbers
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
    class="-mx-1 mt-1 rounded-sm px-1 pb-1.5 pt-1 transition-colors duration-75"
    :class="context.focused.value && !context.editing.value ? 'bg-gray-50' : 'bg-gray-100'"
  />
  <button
    v-if="code.length == 0"
    class="absolute bottom-3 z-10 w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
    @click="monacoRef?.focus()"
  >
    +code
  </button>
  <!-- Last output/error (if any) -->
  <div
    v-if="lastExecution && lastExecution.status == ExecutionStatus.Failed && !hideOutput"
    class="relative -mx-1 mb-0.5 w-full rounded-b-sm border-t border-gray-200 px-1 py-1.5 font-mono transition duration-150"
    :class="[
      context.focused.value && !context.editing.value ? 'bg-gray-50' : 'bg-gray-100',
      lastExecution.status == ExecutionStatus.Failed ? 'text-red-600' : 'text-gray-600',
      truncateOutput ? 'max-h-[250px] overflow-y-hidden' : '',
    ]"
    :key="lastExecution?.id"
  >
    {{ context.statement.value?.name }} {{ lastExecution.status.toLowerCase() }}:
    <span class="font-bold">{{ lastExecution.errorNice?.message }}</span>
    <ul class="mt-1 flex flex-col gap-2">
      <!-- Error traceback -->
      <li
        v-for="(frame, i) of lastExecution.errorNice?.traceback"
        :key="i"
        class="flex max-w-full flex-col overflow-hidden py-0.5 hover:bg-red-100"
      >
        <span>
          <a class="underline underline-offset-4">{{ frame.filename }}:{{ frame.lineno }}</a> {{ frame.name }}
        </span>
        <span class="mx-2 mt-0.5" :class="i == 0 ? 'font-bold' : ''"> > {{ frame.line }} </span>
        <!-- Locals -->
        <span
          v-if="Object.keys(frame.locals).length > 0"
          class="mx-2 mt-0.5 grid grid-cols-4 border border-red-600 p-2"
        >
          <template v-for="key in Object.keys(frame.locals)" :key="key">
            <span>{{ key }}</span>
            <span class="col-span-3 w-full">{{ frame.locals[key] }}</span>
          </template>
        </span>
      </li>
    </ul>
    <!-- If truncating, button overlay with fade gradient -->
    <button
      v-if="truncateOutput"
      class="absolute bottom-0 left-0 flex h-12 w-full items-end justify-center bg-gradient-to-t from-gray-100 to-transparent pb-2"
      @click="truncateOutput = false"
    >
      <ChevronDoubleDownIcon class="h-4 w-4 text-gray-400" />
    </button>
    <button v-else class="flex w-full flex-row justify-center bg-gray-100 pt-0.5" @click="truncateOutput = true">
      <ChevronDoubleUpIcon class="h-4 w-4 text-gray-400" />
    </button>
  </div>
</template>
