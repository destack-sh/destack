<script lang="ts" setup>
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import FunctionType from "@/components/statements/FunctionType.vue";
import InlineActions from "@/components/statements/StatementActions.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus } from "@/gql/graphql";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { ACTIVE_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { TypeFlag } from "@/state/module";
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
  CubeTransparentIcon,
} from "@heroicons/vue/24/outline";
import { nextTick, computed, ref, type Ref, watch } from "vue";
import StatementTags from "@/components/statements/StatementTags.vue";
import { useActiveScroll } from "@/composables/useScroll";
import RunTile from "@/components/tiles/RunTile.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { getRunStatusColor } from "@/state/session";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import TypedStatementDeclaration from "@/components/statements/TypedStatementDeclaration.vue";
import StatementTriggers from "@/components/statements/StatementTriggers.vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void; (e: "toggleActions"): void }>();

const context = useStatementContext();
const editor = useEditorContext();
const standalone = context.standalone;

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
const currentRun = computed(() => runs.value[0]);

const inputs = computed(() => context.fields.value.filter((f) => !(f.flags & TypeFlag.IsOutput)));
const outputs = computed(() => context.fields.value.filter((f) => f.flags & TypeFlag.IsOutput));
const numCodeLines = computed(() => code.value.split("\n").length);

const declarationRef: Ref<InstanceType<typeof TypedStatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionType> | null> = ref(null);
const outputRef = ref<HTMLDivElement | null>(null);
const runTileRef: Ref<InstanceType<typeof RunTile> | null> = ref(null);
const hasTypes = computed(() => context.fields.value.length > 0);
const addingTypes = ref(false);
const showOutput: Ref<"logs" | "trace" | "error" | null> = ref(context.standalone.value ? "logs" : null);

watch(
  () => currentRun.value?.status,
  () => {
    if (currentRun.value?.status == RunStatus.Failed) {
      showOutput.value = "error";
    }
  }
);

const preparingRun = ref(false);
const isCurrentRunActive = computed(
  () =>
    preparingRun.value ||
    (currentRun.value != null &&
      ACTIVE_RUN_STATUSES.includes(currentRun.value?.status) &&
      currentRun.value?.status != RunStatus.Aborting)
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
      label: "Include type",
      icon: CubeTransparentIcon,
      action: () => {
        unfoldIfFolded();
        context.createUnionField();
      },
      hideInline: true,
    },
  ];
  if (isCurrentRunActive.value) {
    inlineActions.push({
      label: "Stop",
      icon: StopIcon,
      action: async () => await cancel(),
    });
  } else {
    inlineActions.push({
      label: "Run",
      icon: PlayIcon,
      disabled: hasTypes.value, // needs parameters
      action: async () => await run(),
    });
  }
  inlineActions.push({
    label: showOutput.value ? "Hide output" : "Show output",
    disabled: currentRun.value == null,
    icon: showOutput.value ? EyeIcon : EyeSlashIcon,
    action: async () => {
      if (showOutput.value == null) {
        showOutput.value = "logs";
      } else {
        showOutput.value = null;
      }
    },
  });
  inlineActions.push({
    label: "Launch",
    icon: WindowIcon,
    action: () => {
      const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
      bench.openRun(context.statement.value, { group: nextGroup, focus: true });
    },
  });

  return inlineActions;
});
context.setCustomActions(extraActions);

async function run() {
  if (isCurrentRunActive.value) return;
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
    showOutput.value = "logs";
    const { result: runPromise } = await sessions.run(context.statement.value);
    const { logs } = await runPromise;
    if (logs != null) {
      nextTick(() => runTileRef.value?.addLogs(logs));
    }
  }
}

async function cancel() {
  if (!isCurrentRunActive.value) return;
  await sessions.cancel(currentRun.value);
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
      <TypedStatementDeclaration
        ref="declarationRef"
        @navigate-down="(typeRef?.focus ?? monacoRef?.focus ?? context.navigateDown)()"
      />
      <StatementTags ref="tagsRef" class="ml-1.5" />
      <StatementTriggers class="ml-1.5" />
    </div>
    <!-- Meta info & controls -->
    <div
      class="group/info flex flex-shrink-0 flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value || isCurrentRunActive ? '' : 'opacity-0'"
    >
      <!-- Run time -->
      <span
        v-if="!hasTypes && currentRun != null"
        :class="[preparingRun ? 'text-gray-400' : getRunStatusColor(currentRun.status, { gray: 'text-gray-400' })]"
      >
        <BusySpinnerIcon v-if="preparingRun || currentRun.status == RunStatus.Queued" class="h-4 w-4 animate-spin" />
        <span v-else>{{ sessions.getDurationFormatted(currentRun) }}</span>
      </span>
      <!-- Cache info -->
      <RunCacheInfo v-if="!hasTypes && currentRun != null" :run="currentRun" class="relative mr-0.5 py-1" />
      <!-- Age -->
      <span
        v-if="!hasTypes"
        :class="[
          preparingRun ? 'text-gray-400' : getRunStatusColor(currentRun?.status, { gray: 'text-gray-400' }),
          currentRun?.updatedAt ? 'opacity-100' : 'opacity-0',
        ]"
      >
        {{ now.getTimeFromNowString(currentRun?.updatedAt) }}</span
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
    @toggle-actions="emit('toggleActions')"
    language="python"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class="-mx-1 mt-0.5 min-h-[32px] rounded-t-sm border border-orange-900 border-opacity-[15%] px-1 pb-1.5 pt-1 transition-colors duration-150"
    :class="[showOutput != null ? '' : 'rounded-b-sm']"
  />
  <!-- Last output: logs/trace/error -->
  <RunTile
    ref="runTileRef"
    v-if="!hasTypes && currentRun != null && !folded && showOutput"
    class="relative -mx-1 mb-0.5 w-full rounded-b-sm border border-t-0 border-gray-200 px-3 py-1.5 transition duration-150"
    :project-id="bench.projectId"
    :project-version-id="bench.projectVersionId"
    :run="currentRun"
    :key="currentRun?.id"
    :view="showOutput"
    show-controls
  />
</template>
