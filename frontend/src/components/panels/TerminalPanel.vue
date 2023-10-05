<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, TerminalPanel } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import {
  SESSION_ACCESS_LEVELS,
  SESSION_ACCESS_LEVEL_NAME,
  SessionAccessLevel,
  useTerminal,
  getRunStatusIconSolid,
  getRunStatusColor,
} from "@/state/session";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import { ChevronDoubleRightIcon, ChevronRightIcon } from "@heroicons/vue/24/outline";
import { ArrowRightIcon, PlayIcon } from "@heroicons/vue/24/solid";
import { Bars3BottomLeftIcon, CodeBracketIcon } from "@heroicons/vue/24/solid";
import { nextTick, computed, ref, type Ref } from "vue";

const props = defineProps<{ panel: PanelContext<TerminalPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const terminal = useTerminal();
const now = useTimeFromNow();

const input: Ref<string> = ref(panel.value.input ?? "");
const inputSync = syncProperty({
  read: () => (input.value = panel.value.input ?? ""),
  write: () => (panel.value.input = input.value),
  debounceMs: 500,
});
const inputRef: Ref<InstanceType<typeof MonacoEditor | typeof AnnotatedText> | null> = ref(null);
const focusedRunId = ref<string | null>(null);
const expandedRunIds = ref<string[]>([]);

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
}

function run() {
  if ((input.value.trim() ?? "").length == 0) {
    return; // nothing to run
  }
  if (panel.value.inputMode == "code") {
    const { run, result } = terminal.runCode(input.value, undefined);
    expandedRunIds.value.push(run.id); // always show full run if it was made here
    result.then(() => {
      input.value = "";
      inputSync.onLocalWrite();
    });
  } else {
    terminal.runText(input.value);
  }
}

defineExpose({
  focus,
});
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }">
    <PanelHeader
      class="border-b border-orange-900 border-opacity-[12%] bg-gray-50"
      :editing="false"
      :thing="null"
      :actions="[]"
      :path="[]"
      :self="-1"
    />
    <!-- History -->
    <div
      class="mx-auto flex max-h-full w-full max-w-full flex-1 overflow-y-auto pt-8 text-sm"
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
        class="flex flex-col border-t border-orange-900/[15%] bg-white py-2"
      >
        <!-- Header -->
        <div class="flex flex-row items-start justify-between pl-4 pr-6" :class="[getRunStatusColor(run.status)]">
          <div class="flex flex-row">
            <!-- Status -->
            <span class="py-0.5">
              <component
                :is="getRunStatusIconSolid(run.status)"
                class="h-4 w-4"
                :class="[getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
              />
            </span>
            <!-- Scope -->
            <span class="ml-2">{{ module.path.value }}</span>
            <!-- Duration -->
            <span v-if="run.startedAt != null" class="ml-1">
              {{ run.terminatedAt != null ? "in" : "for" }}
              {{
                run.duration != null
                  ? formatDuration(run.duration * 1000)
                  : now.getTimeFromNowString(run.startedAt, { useNow: false })
              }}
            </span>
          </div>
          <!-- Extra info & controls -->
          <div class="flex select-none flex-row gap-1.5 text-gray-400">
            <!-- Run ID -->
            <button class="font-mono underline-offset-2 hover:underline" @click="bench.openViewRun(run.id)">
              #{{ getUUIDFromGlobalID(run.id).slice(-7, -1) }}
            </button>
            <!-- From -->
            <span class="text-gray-400">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
          </div>
        </div>
        <!-- Body -->
        <div class="flex w-full flex-col pl-10 pr-4">
          <MonacoEditor :model-value="code" readonly hide-line-numbers language="python" :focused="panel.focused" />
          <ErrorTraceback v-if="run.errorNice != null" hide-preamble :error-nice="run.errorNice" class="" />
          <div v-if="expandedRunIds.includes(run.id)">
            <LogsTile
              :project-id="(bench.projectId as string)"
              :project-version-id="(bench.projectVersionId as string)"
              :run-id="run.id"
              :session-id="run.session?.id"
              hide-if-empty
            />
          </div>
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
          <div class="flex flex-row items-start text-gray-500">
            <span class="px-1 py-0.5">
              <ChevronDoubleRightIcon class="h-4 w-4" />
            </span>
            <!-- Current context/path & mode -->
            <span class="ml-0.5">{{ module.path.value }}</span>
            <!-- Access level -->
            <button
              class="hover rounded-sm px-1.5"
              :class="[
                panel.accessLevel < SessionAccessLevel.Delete
                  ? '   ring-gray-600/10 hover:bg-orange-100'
                  : 'bg-red-100 font-semibold text-red-900  hover:bg-red-200',
              ]"
              @click="
                () => {
                  const levels = SESSION_ACCESS_LEVELS;
                  panel.accessLevel = levels[(levels.indexOf(panel.accessLevel) + 1) % levels.length];
                }
              "
            >
              can {{ SESSION_ACCESS_LEVEL_NAME[panel.accessLevel]?.toLowerCase() }}
            </button>
          </div>
          <!-- Body -->
          <div class="flex flex-row">
            <!-- Text/Code toggle -->
            <button
              @click="panel.toggleInputMode()"
              class="flex h-fit flex-row rounded-sm px-1 py-[2px] text-orange-600 hover:bg-orange-100"
            >
              <component :is="panel.inputMode == 'code' ? CodeBracketIcon : Bars3BottomLeftIcon" class="h-4 w-4" />
            </button>
            <!-- Input -->
            <div class="relative ml-0.5 min-h-[22px] w-full">
              <AnnotatedText v-if="panel.inputMode == 'text'" ref="inputRef" v-model="input" @click.stop @enter="run" />
              <MonacoEditor
                v-else
                ref="inputRef"
                v-model="input"
                @update:model-value="onInputWrite"
                :focused="panel.focused"
                language="python"
                hide-line-numbers
                enter-is-execute
                @click.stop
                @execute="run"
                @navigate-up="navigateInputUp()"
                @navigate-down="navigateInputDown()"
              />
            </div>
          </div>
        </div>
        <!-- Run -->
        <button class="self-start rounded-sm px-1.5 py-1 hover:bg-orange-100" @click="run">
          <PlayIcon class="h-6 w-6 text-orange-600" />
        </button>
      </div>
    </div>
  </div>
</template>
