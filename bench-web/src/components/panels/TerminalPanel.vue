<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import CodeBlock from "@/components/basic/CodeBlock.vue";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus } from "@/gql/graphql";
import { useBenchState, type PanelContext, TerminalPanel } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import {
  SESSION_ACCESS_LEVELS,
  SESSION_ACCESS_LEVEL_NAME,
  SessionAccessLevel,
  getRunStatusColor,
  useCurrentSessions,
  ACTIVE_RUN_STATUSES,
} from "@/state/session";
import { useTerminal } from "@/state/terminal";
import { IdentifierType, toGlobalId, toPyIdentifier } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import { ChevronDoubleDownIcon, SparklesIcon } from "@heroicons/vue/24/outline";
import { PlayIcon, StopIcon } from "@heroicons/vue/24/solid";
import { nextTick, computed, ref, type Ref, watch } from "vue";

const props = defineProps<{ panel: PanelContext<TerminalPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "focus"): void;
}>();

const bench = useBenchState();
const module = useCurrentModule();
const session = useCurrentSessions();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const terminal = useTerminal();
const visibleHistory = computed(() => {
  if (panel.value.clearedAt == null) return terminal.runs.value;
  else return terminal.runs.value?.filter((r) => r.run.createdAt > (panel.value.clearedAt ?? ""));
});
const now = useTimeFromNow(1000);
const loading = computed(() => terminal.loading.value || module.loading.value);

const input: Ref<string> = ref(panel.value.code ?? "");
const inputSync = syncProperty({
  read: () => (input.value = panel.value.code ?? ""),
  write: () => (panel.value.code = input.value),
  debounceMs: 500,
});
const inputRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
const logsTileRefs = ref<Record<string, InstanceType<typeof LogsTile> | null>>({});
const scope = computed(() => bench.lastActiveFileCk);
const scopePaths = computed(() => {
  const scopes = [scope.value, ...(visibleHistory.value?.map((r) => r.scope) ?? [])];
  const scopePaths: Record<string, string> = {};
  for (const scope of scopes) {
    if (scope == null || scopePaths[scope] != null) continue;
    const nodePath = module.nodePathOf(scope);
    if (nodePath?.some((n) => (n.name ?? "").trim().length == 0)) {
      continue;
    }
    const scopePath = nodePath?.map((n) => toPyIdentifier(n.name ?? "", IdentifierType.PATH)).join(".");
    if (scopePath != null) scopePaths[scope] = scopePath;
  }
  return scopePaths;
});
const focusedRunId = ref<string | null>(null);
const expandedRunIds = ref<string[]>([]);
const lastRunActive = computed(() =>
  panel.value.lastRunId == null ? false : session.isActive({ id: panel.value.lastRunId })
);
const canRun = computed(() => (input.value.trim() ?? "").length > 0);

function focus(f: "first" | "last" = "last") {
  inputRef.value?.focus?.("last");
}

function onInputWrite() {
  inputSync.onLocalWrite();
  focusedRunId.value = null;
}

const lastClearedInput: Ref<string | null> = ref(null);

function navigateInputUp() {
  // scroll backwards to last input
  if (!visibleHistory.value) return;
  const currentRunIndex = visibleHistory.value.findIndex((r) => r.run.id == focusedRunId.value);
  if (currentRunIndex == -1) {
    lastClearedInput.value = input.value;
  }
  if (currentRunIndex < visibleHistory.value.length - 1) {
    const nextRun = visibleHistory.value[currentRunIndex + 1];
    focusedRunId.value = nextRun.run.id;
    input.value = nextRun.code;
  }
  nextTick(() => focus("last"));
}

function navigateInputDown() {
  // scroll forwards to next input / clear
  if (!visibleHistory.value && lastClearedInput.value == null) return;
  const currentRunIndex = visibleHistory.value.findIndex((r) => r.run.id == focusedRunId.value);
  if (currentRunIndex > 0) {
    const nextRun = visibleHistory.value[currentRunIndex - 1];
    focusedRunId.value = nextRun.run.id;
    input.value = nextRun.code;
  } else {
    focusedRunId.value = null;
    input.value = lastClearedInput.value ?? "";
  }
  nextTick(() => focus("last"));
}

async function run() {
  // handle special commands to clear / restore history
  if (input.value == "cls" || input.value == "clear") {
    input.value = "";
    panel.value.clearHistory();
    return;
  } else if (input.value == "restore" || input.value == "history") {
    input.value = "";
    panel.value.restoreHistory();
    return;
  }

  if (!canRun.value) return;
  // actually run
  const { run, firstResult } = terminal.runCode(input.value, {
    scope: bench.lastActiveFileCk ?? module.allFiles.value[0]?.ck,
    accessLevel: panel.value.accessLevel,
    tags: ["test"],
  });
  firstResult.then((r) => logsTileRefs.value[run.id]?.addLogs(r.logs ?? []));
  focusedRunId.value = null;
  expandedRunIds.value.push(run.id); // always show full run if it was made here
  panel.value.lastRunId = run.id;
  input.value = "";
  inputSync.flushNow();
}

defineExpose({
  focus,
});
</script>
<template>
  <div class="relative flex flex-col" :style="{ height: panelSize.height + 'px' }">
    <!-- History -->
    <div
      class="mx-auto flex h-full max-h-full w-full max-w-full flex-1 overflow-x-hidden overflow-y-scroll text-sm"
      :class="loading || visibleHistory?.length == 0 ? 'flex-col items-center justify-center' : 'flex-col-reverse'"
    >
      <!-- Loading / empty state -->
      <div v-if="loading || visibleHistory?.length == 0" class="self-center justify-self-center">
        <BusySpinnerIcon v-if="loading" class="mx-auto h-8 w-8 animate-spin text-white" />
        <span v-else class="text-gray-500">No terminal history</span>
      </div>
      <!-- Previous runs -->
      <div
        v-for="{ run, code, scope, generatedFrom, generatedIn } in loading ? [] : visibleHistory"
        :key="run.id"
        class="opacity-150 w-full border-l-4 border-t border-orange-900/[15%] py-1.5 transition-colors"
        :class="[run.status == RunStatus.Failed ? 'border-l-red-300 bg-red-100' : 'border-l-white bg-white']"
      >
        <div class="mx-auto flex flex-col" :style="{ ...panel.contentWidthAsMaxWidth }">
          <!-- Header -->
          <div class="flex flex-row items-start justify-between pl-4 pr-4 font-mono text-gray-400">
            <!-- Scope -->
            <span class=""
              >{{ module.path.value }}<template v-if="scope">.{{ scopePaths[scope] }}</template>
            </span>
            <!-- Extra info & controls -->
            <div class="flex select-none flex-row gap-2 text-gray-400">
              <!-- Generated from -->
              <div
                v-if="generatedFrom"
                class="group flex max-w-[200px] flex-row items-center font-normal hover:cursor-pointer"
                @click.stop="bench.openViewRun({ id: toGlobalId('Run', generatedIn) }, { focus: true })"
              >
                <SparklesIcon class="h-4 w-4 flex-shrink-0 text-gray-400" />
                <span class="underline-offfset-2 ml-1 truncate text-gray-400 group-hover:underline">
                  {{ generatedFrom }}
                </span>
              </div>
              <!-- Run ID -->
              <button
                class="flex-shrink-0 font-mono underline-offset-2 hover:underline"
                @click.stop="bench.openViewRun(run, { focus: true })"
                :class="[
                  !ACTIVE_RUN_STATUSES.includes(run.status) && run.status != RunStatus.Completed
                    ? getRunStatusColor(run.status)
                    : '',
                ]"
              >
                <!-- Duration -->
                <span v-if="run.startedAt != null" class="">
                  {{ session.getDurationFormatted(run) }}
                </span>
                <!-- From -->
                <span class="ml-1">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
              </button>
              <!-- Copy to current -->
              <button class="flex-shrink-0 rounded-sm text-gray-400 hover:bg-orange-100" @click="input = code">
                <ChevronDoubleDownIcon class="h-4 w-4" />
              </button>
            </div>
          </div>
          <!-- Body -->
          <div class="relative flex w-full max-w-full flex-col pl-4 pr-4">
            <CodeBlock :model-value="code" wrap language="python" />
            <LogsTile
              v-if="expandedRunIds.includes(run.id)"
              :ref="(ref: any) => (logsTileRefs[run.id] = ref)"
              :project-id="(bench.projectId as string)"
              :project-version-id="(bench.projectVersionId as string)"
              :run-id="run.id"
              :session-id="run.session?.id"
              hide-if-empty
              hide-metadata
              always-expand
              :live="run.terminatedAt == null"
              :class="[
                (logsTileRefs[run.id]?.logs?.length ?? 0) > 0 ? 'mt-1 border-t border-orange-900/[15%] py-1' : '',
              ]"
            />
            <ErrorTraceback
              v-if="run.errorNice != null && expandedRunIds.includes(run.id)"
              hide-preamble
              :error-nice="run.errorNice"
              class="mt-1 border-t border-orange-900/[15%] py-1"
            />
          </div>
        </div>
      </div>
    </div>
    <!-- Input (bottom)-->
    <div class="w-full flex-shrink-0 border-t border-orange-900/[15%] bg-white pb-3 pr-3 pt-2.5" @click="focus()">
      <div class="mx-auto flex w-full flex-row gap-x-2 text-sm text-gray-900" :style="panel.contentWidthAsMaxWidth">
        <div class="flex flex-1 flex-col pl-4 pr-4">
          <!-- Header -->
          <div class="flex flex-row items-start font-mono text-orange-600">
            <!-- Current context/path & mode -->
            <span class="font-semibold"
              >{{ module.path.value }}<template v-if="scope">.{{ scopePaths[scope] }}</template>
            </span>
            <!-- Access level -->
            <button
              class="hover ml-1 rounded-sm px-0.5"
              :class="[
                panel.accessLevel < SessionAccessLevel.Delete
                  ? 'ring-gray-600/10 hover:bg-orange-100'
                  : 'bg-red-100 font-semibold text-red-900  hover:bg-red-200',
              ]"
              @click="
                () => {
                  const levels = SESSION_ACCESS_LEVELS;
                  panel.accessLevel = levels[(levels.indexOf(panel.accessLevel) + 1) % levels.length];
                }
              "
            >
              (can {{ SESSION_ACCESS_LEVEL_NAME[panel.accessLevel]?.toLowerCase() }})
            </button>
          </div>
          <!-- TODO @UX: assist code completion in terminal -->
          <MonacoEditor
            class="h-full min-h-[22px] w-full"
            ref="inputRef"
            v-model="input"
            @update:model-value="onInputWrite"
            :focused="panel.focused"
            language="python"
            hide-line-numbers
            wrap
            enter-is-execute
            @click.stop="emit('focus')"
            @execute="run"
            @navigate-up="navigateInputUp()"
            @navigate-down="navigateInputDown()"
          />
        </div>
        <!-- Run -->
        <button
          class="flex-shrink-0 self-start rounded-sm px-1.5 py-1.5 transition-colors duration-150 hover:bg-orange-100"
          :class="lastRunActive || canRun ? 'text-orange-600' : 'text-gray-400'"
          @click="lastRunActive && panel.lastRunId != null ? session.kill({ id: panel.lastRunId }) : run()"
        >
          <PlayIcon v-if="!lastRunActive" class="h-6 w-6" />
          <StopIcon v-else class="h-6 w-6" />
        </button>
      </div>
    </div>
    <span v-if="bench.debug" class="absolute left-0 top-0 ml-2 bg-red-200 bg-opacity-50 text-xs text-gray-900">
      {{ bench.focusedPanelId == panel.id ? "(focused)" : "" }}
      cleared:{{ panel.clearedAt }}
    </span>
  </div>
</template>
