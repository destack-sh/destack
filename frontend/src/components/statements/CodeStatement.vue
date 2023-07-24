<script lang="ts" setup>
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import StatementDeclaration from "@/components/statements/StatementDeclaration.vue";
import FunctionType from "@/components/statements/FunctionType.vue";
import InlineActions from "@/components/statements/StatementActions.vue";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { RunStatus, type Run, type LogEntry } from "@/gql/graphql";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { RUN_TERMINAL_STATES, useCurrentSessions } from "@/state/session";
import { TypeFlag, newRunId, newSessionId } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { useStatementContext } from "@/state/statement";
import {
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
import { nextTick, computed, ref, type Ref, watch } from "vue";
import { DateTime } from "luxon";
import StatementTags from "@/components/statements/StatementTags.vue";
import { useActiveScroll } from "@/composables/useScroll";
import RunTile from "@/components/tiles/RunTile.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";

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

const bench = useBenchState();
const sessions = useCurrentSessions();
const runs = sessions.runsOf(context.statement.value);

const inputs = computed(() => context.fields.value.filter((f) => !(f.flags & TypeFlag.IsOutput)));
const outputs = computed(() => context.fields.value.filter((f) => f.flags & TypeFlag.IsOutput));
const numCodeLines = computed(() => code.value.split("\n").length);

const lastRun = computed(() => runs.value[0]);

const declarationRef: Ref<InstanceType<typeof StatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionType> | null> = ref(null);
const outputRef = ref<HTMLDivElement | null>(null);
const runTileRef: Ref<InstanceType<typeof RunTile> | null> = ref(null);
const hasTypes = computed(() => context.fields.value.length > 0);
const addingTypes = ref(false);
const showOutput: Ref<"logs" | "trace" | "error" | null> = ref(context.standalone.value ? "logs" : null);

watch(
  () => lastRun.value?.status,
  () => {
    if (lastRun.value?.status == RunStatus.Failed) {
      showOutput.value = "error";
    }
  }
);

const preparingRun = ref(false);
const runActive = computed(
  () => preparingRun.value || (lastRun.value != null && !RUN_TERMINAL_STATES.includes(lastRun.value?.status))
);

useActiveScroll(outputRef);

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
      active: runActive.value,
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
  if (runActive.value) {
    inlineActions.push({
      label: "Cancel",
      icon: StopIcon,
      action: async () => await cancel(),
    });
  } else {
    inlineActions.push({
      label: showOutput.value ? "Hide output" : "Show output",
      disabled: lastRun.value == null,
      icon: showOutput.value ? EyeIcon : EyeSlashIcon,
      action: async () => {
        if (showOutput.value == null) {
          showOutput.value = "logs";
        } else {
          showOutput.value = null;
        }
      },
    });
  }

  return inlineActions;
});
context.setCustomActions(extraActions);

async function run() {
  if (runActive.value) return;
  if (hasTypes.value) {
    const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
    bench.openRun(context.statement.value, { group: nextGroup, focus: true });
  } else {
    try {
      // TODO @Robustness: ensure that executed code is always exact same as in editor (wait for revision?)
      await codeSync.flushNow(); // flush any pending changes to the code (which is debounced)
      preparingRun.value = true;
    } finally {
      preparingRun.value = false;
    }
    await sessions.run(context.statement.value);
    showOutput.value = "logs";
  }
}

async function cancel() {
  if (!runActive.value) return;
  await sessions.cancel(lastRun.value);
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
  loading: computed(() => false), // runs may be loading, but does not affect layout because output/state hidden by default
});
</script>
<template>
  <!-- Header -->
  <div class="flex flex-row justify-between">
    <!-- Declaration -->
    <div class="flex flex-row items-center">
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
      :class="context.focused.value || runActive ? '' : 'opacity-0'"
    >
      <!-- Run time -->
      <span
        v-if="!hasTypes && lastRun != null"
        :class="[lastRun?.status != RunStatus.Failed || preparingRun ? 'text-gray-400' : 'text-red-600']"
      >
        {{
          formatDurationSeconds(
            (lastRun?.duration ?? now.now.value.diff(DateTime.fromISO(lastRun.startedAt)).as("seconds")) * 1000
          )
        }}
      </span>
      <!-- Cache info -->
      <RunCacheInfo v-if="!hasTypes && lastRun != null" :run="lastRun" class="relative mr-0.5 py-1" />
      <!-- Age -->
      <span
        v-if="!hasTypes"
        :class="[
          lastRun?.status != RunStatus.Failed || preparingRun ? 'text-gray-400' : 'text-red-600',
          lastRun?.updatedAt ? 'opacity-100' : 'opacity-0',
        ]"
      >
        {{ now.getTimeFromNowString(lastRun?.updatedAt) }}</span
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
    :class="[showOutput != null ? '' : 'rounded-b-sm']"
  />
  <!-- Last output: logs/trace/error -->
  <RunTile
    ref="runTileRef"
    v-if="!hasTypes && lastRun != null && !folded && showOutput"
    class="relative -mx-1 mb-0.5 w-full rounded-b-sm border border-t-0 border-gray-200 px-3 py-1.5 transition duration-150"
    :project-id="bench.projectId"
    :project-version-id="bench.projectVersionId"
    :run="lastRun"
    :key="lastRun?.id"
    :view="showOutput"
    show-controls
  />
</template>
