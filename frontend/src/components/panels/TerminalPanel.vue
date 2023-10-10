<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus } from "@/gql/graphql";
import { useBenchState, type PanelContext, TerminalPanel, type PanelAction } from "@/state/bench";
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
import { IdentifierType, toPyIdentifier } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import { ArrowRightIcon, PlayIcon, SparklesIcon, StopIcon } from "@heroicons/vue/24/solid";
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
const now = useTimeFromNow(1000);

const input: Ref<string> = ref(panel.value.code ?? "");
const inputSync = syncProperty({
  read: () => (input.value = panel.value.code ?? ""),
  write: () => (panel.value.code = input.value),
  debounceMs: 500,
});
const inputRef: Ref<InstanceType<typeof MonacoEditor | typeof AnnotatedText> | null> = ref(null);
const logsTileRefs = ref<Record<string, InstanceType<typeof LogsTile> | null>>({});
const scope = computed(() => bench.lastActiveFileCk);
const scopePaths = computed(() => {
  const scopes = [scope.value, ...(terminal.runs.value?.map((r) => r.scope) ?? [])];
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

function navigateInputUp() {
  // scroll backwards to last input
  if (!terminal.runs.value) return;
  const currentRunIndex = terminal.runs.value.findIndex((r) => r.run.id == focusedRunId.value);
  if (currentRunIndex < terminal.runs.value.length - 1) {
    const nextRun = terminal.runs.value[currentRunIndex + 1];
    focusedRunId.value = nextRun.run.id;
    input.value = nextRun.code;
  }
  nextTick(() => focus("last"));
}

function navigateInputDown() {
  // scroll forwards to next input / clear
  if (!terminal.runs.value) return;
  const currentRunIndex = terminal.runs.value.findIndex((r) => r.run.id == focusedRunId.value);
  if (currentRunIndex > 0) {
    const nextRun = terminal.runs.value[currentRunIndex - 1];
    focusedRunId.value = nextRun.run.id;
    input.value = nextRun.code;
  } else {
    focusedRunId.value = null;
    input.value = "";
  }
  nextTick(() => focus("last"));
}

async function run() {
  if (!canRun.value) return;
  const { run, result } = terminal.runCode(input.value, {
    scope: bench.lastActiveFileCk ?? undefined,
    accessLevel: panel.value.accessLevel,
    tags: ["test"],
  });
  result.then((r) => logsTileRefs.value[run.id]?.addLogs(r.logs ?? []));
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
      class="mx-auto flex max-h-full w-full max-w-full flex-1 overflow-x-hidden overflow-y-scroll text-sm"
      :class="
        terminal.loading.value || terminal.totalCount.value == 0
          ? 'flex-col items-center justify-center'
          : 'flex-col-reverse'
      "
    >
      <!-- Loading / empty state -->
      <div v-if="terminal.loading.value || terminal.totalCount.value == 0" class="self-center justify-self-center">
        <BusySpinnerIcon v-if="terminal.loading.value" class="mx-auto h-5 w-5 animate-spin text-white" />
        <span v-else class="text-gray-500">No terminal history</span>
      </div>
      <!-- Previous runs -->
      <div
        v-for="{ run, code, scope } in terminal.runs.value"
        :key="run.id"
        class="opacity-150 border-l-4 border-t border-orange-900/[15%] py-1.5 transition-colors"
        :class="[run.status == RunStatus.Failed ? 'border-l-red-300 bg-red-100' : 'border-l-white bg-white']"
      >
        <div class="mx-auto flex flex-col" :style="{ ...panel.contentWidthAsMaxWidth }">
          <!-- Header -->
          <div class="flex flex-row items-start justify-between pl-4 pr-4 font-mono text-gray-400">
            <!-- Scope -->
            <span class="font-semibold"
              >{{ module.path.value }}<template v-if="scope">.{{ scopePaths[scope] }}</template>
            </span>
            <!-- Extra info & controls -->
            <div class="flex select-none flex-row gap-1.5 text-gray-400">
              <!-- Run ID -->
              <button
                class="font-mono underline-offset-2 hover:underline"
                @click="bench.openViewRun(run, { focus: true })"
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
            </div>
          </div>
          <!-- Body -->
          <div class="flex w-full max-w-full flex-col pl-4 pr-4">
            <MonacoEditor
              :model-value="code"
              wrap
              readonly
              hide-line-numbers
              language="python"
              :focused="panel.focused"
            />
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
              v-if="run.errorNice != null"
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
            class="min-h-[22px] w-full"
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
  </div>
</template>
