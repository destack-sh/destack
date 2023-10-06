<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { RunStatus } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
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
import { IdentifierType, getUUIDFromGlobalID, toPyIdentifier } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import { ChevronDoubleRightIcon, ChevronRightIcon } from "@heroicons/vue/24/outline";
import { ArrowRightIcon, PlayIcon, StopIcon } from "@heroicons/vue/24/solid";
import { Bars3BottomLeftIcon, CodeBracketSquareIcon } from "@heroicons/vue/24/solid";
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

const input: Ref<string> = ref(panel.value.input ?? "");
const inputSync = syncProperty({
  read: () => (input.value = panel.value.input ?? ""),
  write: () => (panel.value.input = input.value),
  debounceMs: 500,
});
const inputRef: Ref<InstanceType<typeof MonacoEditor | typeof AnnotatedText> | null> = ref(null);
const logsTileRefs = ref<Record<string, InstanceType<typeof LogsTile> | null>>({});
const scopePath = computed(() => {
  if (bench.lastActiveFileCk == null) return null;
  const nodePath = module.nodePathOf(bench.lastActiveFileCk);
  if (nodePath?.some((n) => (n.name ?? "").trim().length == 0)) {
    return null;
  }
  return nodePath?.map((n) => toPyIdentifier(n.name ?? "", IdentifierType.PATH)).join(".");
});
const focusedRunId = ref<string | null>(null);
const expandedRunIds = ref<string[]>([]);
const lastRunActive = computed(() =>
  panel.value.lastRunId == null ? false : session.isActive({ id: panel.value.lastRunId })
);
const convertingText = ref(false);

function focus(f: "first" | "last" = "last") {
  inputRef.value?.focus?.("last");
}

function onInputWrite() {
  inputSync.onLocalWrite();
  focusedRunId.value = null;
}

function toggleLanguage() {
  inputSync.onLocalWrite();
  if (panel.value.inputMode == "code") {
    panel.value.inputMode = "text";
  } else {
    panel.value.inputMode = "code";
  }
  nextTick(() => {
    // nocheckin: focus code (from text) doesn't work in toggle
    inputRef.value?.focus?.("last");
  });
}

// toggle language depending on input
watch(
  () => [panel.value.inputMode, input.value],
  () => {
    if (
      panel.value.inputMode == "code" &&
      (input.value.startsWith("# ") || input.value.startsWith("// ")) &&
      input.value.split("\n").length == 1
    ) {
      input.value = input.value.slice(2);
      toggleLanguage();
    } else if (panel.value.inputMode == "text" && input.value.startsWith("`")) {
      input.value = input.value.slice(1);
      toggleLanguage();
    }
  }
);

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
  if ((input.value.trim() ?? "").length == 0) {
    return; // nothing to run
  }
  if (panel.value.inputMode == "code") {
    const { run, result } = terminal.runCode(input.value, {
      scope: bench.lastActiveFileCk ?? undefined,
      accessLevel: panel.value.accessLevel,
    });
    result.then((r) => logsTileRefs.value[run.id]?.addLogs(r.logs ?? []));
    focusedRunId.value = null;
    expandedRunIds.value.push(run.id); // always show full run if it was made here
    panel.value.lastRunId = run.id;
    input.value = "";
    inputSync.flushNow();
  } else {
    convertingText.value = true;
    try {
      const { code } = await terminal.runText(input.value, {
        runMode: "approve",
        accessLevel: panel.value.accessLevel,
      });
      input.value = `# ${input.value}\n${code}`;
      inputSync.onLocalWrite();
      panel.value.inputMode = "code";
      nextTick(() => focus("last"));
    } finally {
      convertingText.value = false;
    }
  }
}

defineExpose({
  focus,
});
</script>
<template>
  <div class="relative flex flex-col" :style="{ height: panelSize.height + 'px' }">
    <!-- History -->
    <div
      class="mx-auto flex max-h-full w-full max-w-full flex-1 overflow-y-auto text-sm"
      :class="
        terminal.loading.value || terminal.totalCount.value == 0
          ? 'flex-col items-center justify-center'
          : 'flex-col-reverse'
      "
      :style="{ ...panel.contentWidthAsFixed }"
    >
      <!-- Loading / empty state -->
      <div v-if="terminal.loading.value || terminal.totalCount.value == 0" class="self-center justify-self-center">
        <BusySpinnerIcon v-if="terminal.loading.value" class="mx-auto h-5 w-5 animate-spin text-white" />
        <span v-else class="text-gray-500">No terminal history</span>
      </div>
      <!-- Previous runs -->
      <div
        v-for="{ run, code } in terminal.runs.value"
        :key="run.id"
        class="flex flex-col border-l-4 border-t border-orange-900/[15%] py-2"
        :class="[run.status == RunStatus.Failed ? 'border-l-red-300 bg-red-100' : 'border-l-white bg-white']"
      >
        <!-- Header -->
        <div class="flex flex-row items-start justify-between pl-3 pr-6 font-mono text-gray-400">
          <div class="flex flex-row">
            <!-- Icon -->
            <span class="py-0.5">
              <ChevronDoubleRightIcon class="h-4 w-4" />
            </span>
            <!-- Scope -->
            <span class="ml-2">
              {{ module.path.value }}
            </span>
          </div>
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
        <div class="flex w-full flex-col pl-9 pr-4">
          <MonacoEditor :model-value="code" readonly hide-line-numbers language="python" :focused="panel.focused" />
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
            :class="[(logsTileRefs[run.id]?.logs?.length ?? 0) > 0 ? 'mt-1 border-t border-orange-900/[15%] py-1' : '']"
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
    <!-- Input (bottom)-->
    <div class="w-full flex-shrink-0 border-t border-orange-900/[15%] bg-white" @click="focus()">
      <div
        class="mx-auto flex w-full max-w-full flex-row gap-x-2 bg-white pb-3.5 pl-3 pr-4 pt-3 text-sm text-gray-900"
        :style="panel.contentWidthAsFixed"
      >
        <div class="flex w-full flex-1 flex-col">
          <!-- Header -->
          <div class="flex flex-row items-start font-mono text-orange-600">
            <span class="px-1 py-0.5">
              <ChevronDoubleRightIcon class="h-4 w-4" />
            </span>
            <!-- Current context/path & mode -->
            <span class="ml-0.5 font-semibold"
              >{{ module.path.value }}<template v-if="scopePath">.{{ scopePath }}</template>
            </span>
            <!-- Access level -->
            <button
              class="hover ml-0.5 rounded-sm px-0.5"
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
          <!-- Body -->
          <div class="flex flex-row">
            <!-- Text/Code toggle -->
            <button
              @click="panel.toggleInputMode()"
              class="flex h-fit flex-row rounded-sm px-1 py-[2px] text-orange-600 hover:bg-orange-100"
            >
              <component
                :is="panel.inputMode == 'code' ? CodeBracketSquareIcon : Bars3BottomLeftIcon"
                class="h-4 w-4"
              />
            </button>
            <!-- Input -->
            <div class="relative ml-0.5 min-h-[22px] w-full">
              <AnnotatedText
                v-if="panel.inputMode == 'text'"
                ref="inputRef"
                v-model="input"
                @update:model-value="onInputWrite"
                @click.stop="emit('focus')"
                @enter="run"
                @enter-right="run"
                @toggle-language="toggleLanguage"
                @illegal="toggleLanguage"
              />
              <MonacoEditor
                v-else
                ref="inputRef"
                v-model="input"
                @update:model-value="onInputWrite"
                :focused="panel.focused"
                language="python"
                hide-line-numbers
                enter-is-execute
                @click.stop="emit('focus')"
                @execute="run"
                @navigate-up="navigateInputUp()"
                @navigate-down="navigateInputDown()"
                @toggle-language="toggleLanguage"
              />
            </div>
          </div>
        </div>
        <!-- Run -->
        <button
          class="self-start rounded-sm px-1.5 py-1 text-orange-600 hover:bg-orange-100"
          @click="lastRunActive && panel.lastRunId != null ? session.kill({ id: panel.lastRunId }) : run()"
          :disabled="convertingText"
        >
          <BusySpinnerIcon v-if="convertingText" class="h-6 w-6 animate-spin text-white" />
          <PlayIcon v-else-if="!lastRunActive" class="h-6 w-6" />
          <StopIcon v-else class="h-6 w-6" />
        </button>
      </div>
    </div>
  </div>
</template>
