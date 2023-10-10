<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { formatDuration } from "@/composables/useNow";
import type { Run } from "@/gql/graphql";
import { useBenchState, type StatementHeader, usePanelContext } from "@/state/bench";
import { SessionAccessLevel, useCurrentSessions } from "@/state/session";
import { useTerminal } from "@/state/terminal";
import { makeTextMention, makeTextPlain, renderTextHtml, type TextSpan } from "@/state/text";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { CommandLineIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { PlayIcon, SparklesIcon, StopIcon } from "@heroicons/vue/24/solid";
import { onClickOutside } from "@vueuse/core";
import { ref, nextTick, computed } from "vue";

const props = defineProps<{
  fileCk: string;
}>();
const bench = useBenchState();
const panel = usePanelContext();
const terminal = useTerminal();
const session = useCurrentSessions();

const containerRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof AnnotatedText> | null>(null);
const inputText = ref("");
const canRun = computed(
  // check if there is any meaningful text
  () => inputRef.value?.spans.some((s) => s.type == "text" && s.text.trim() != "") ?? false
);
const active = ref(false);
const expanded = computed(() => active.value || generatingRun.value || generatedCode.value != null);
const generatingRun = ref<Run | null>(null);
const generatedFrom = ref<string | null>(null);
const generatedCode = ref<string | null>(null);
const generatedRun = ref<Run | null>(null);
const generatedCodeCondensed = computed(
  () => generatedCode.value != null && generatedCode.value.split("\n").length < 5
);
const applyingCode = ref(false);

const fullWidth = computed(() => Math.min(panel.size.value.width - 40, 600));

onClickOutside(containerRef, () => {
  if (active.value) {
    inputText.value = "";
    close();
  }
});

async function run() {
  if (!canRun.value) return;
  if (generatingRun.value) {
    cancel();
  }
  discardGenerated();
  try {
    // clear input text from span references (ignore content)
    generatedFrom.value = inputText.value.replace(/<span.*?>/g, "").replace(/<\/span>/g, "");
    const { result, run } = terminal.runTextToCode(inputText.value);
    generatingRun.value = run;
    generatedRun.value = run;
    const { code } = await result;
    if (generatingRun.value?.id != run.id) return; // cancelled or something
    generatedCode.value = code;
  } finally {
    generatingRun.value = null;
  }
}

async function cancel() {
  if (generatingRun.value == null) return;
  const success = await session.kill(generatingRun.value);
  if (!success) {
    console.error("failed to kill run", generatingRun.value);
    generatingRun.value = null;
  }
}

function discardGenerated() {
  generatedCode.value = null;
  generatedFrom.value = null;
  generatedRun.value = null;
}

async function apply() {
  if (generatedCode.value == null || generatedFrom.value == null) return;
  applyingCode.value = true;
  try {
    const { result } = terminal.runCode(generatedCode.value, {
      scope: props.fileCk,
      accessLevel: SessionAccessLevel.Update,
      tags: ["mend"],
    });
    await result;
    discardGenerated();
    inputText.value = "";
    close();
  } catch (e) {
    console.error("failed to apply code", e);
  } finally {
    applyingCode.value = false;
  }
}

function open(text: string, selection?: StatementHeader[], from?: StatementHeader) {
  if (selection) {
    const spans: TextSpan[] = [...selection.map((s) => makeTextMention(s)), makeTextPlain(" " + text)];
    text = renderTextHtml(spans);
  }
  inputText.value = text ?? "";
  if (text && !active.value) {
    run();
  }
  active.value = true;
  nextTick(focus);
}

function close() {
  active.value = false;
}

function focus() {
  inputRef.value?.focus("last");
}

defineExpose({
  open,
});
</script>
<template>
  <div
    ref="containerRef"
    class="flex flex-col-reverse gap-2 text-sm text-gray-900 transition-all duration-150"
    :class="[active ? '' : 'opacity-70 focus-within:opacity-100 hover:opacity-100']"
  >
    <!-- Header/input -->
    <div
      class="relative flex flex-row gap-1 bg-white px-2 py-1.5 shadow-md ring-1 ring-orange-900 transition-all duration-150"
      :class="[
        expanded
          ? 'rounded-sm ring-opacity-40'
          : 'w-10 rounded-2xl ring-opacity-20 hover:cursor-pointer hover:bg-orange-100',
      ]"
      :style="expanded ? { width: fullWidth + 'px' } : {}"
      @click="active ? focus() : open(inputText)"
      @keydown.escape.stop.prevent="discardGenerated(), close()"
    >
      <!-- Open/close button -->
      <button
        class="flex-shrink-0 self-start rounded-sm p-1 text-orange-600 transition-colors duration-150 hover:cursor-pointer hover:bg-orange-100"
        @click="(e) => !expanded || (e.stopPropagation(), close())"
      >
        <SparklesIcon class="h-4 w-4" />
      </button>
      <!-- Actual input -->
      <AnnotatedText
        v-if="expanded"
        ref="inputRef"
        class="min-w-[5px] max-w-full overflow-x-hidden whitespace-pre py-0.5"
        v-model="inputText"
        suppress-shortcuts
        allow-all-characters
        @click.stop
        @enter="run"
        @enter-right="run"
      />
      <span v-if="inputText.length == 0">&nbsp;</span>
      <!-- Run -->
      <span v-if="expanded" class="ml-auto flex flex-shrink-0 items-center self-start">
        <!-- Active -->
        <span v-if="generatingRun != null" class="mr-0.5 text-gray-400">
          {{ session.getDurationFormatted(generatingRun, { hideMillis: true }) }}
        </span>
        <!-- Start/stop -->
        <button
          class="rounded-sm p-1 transition-colors duration-150 hover:bg-orange-100"
          :class="[canRun || generatingRun ? 'text-orange-600' : 'text-gray-400']"
          @click="generatingRun ? cancel() : run()"
        >
          <PlayIcon v-if="!generatingRun" class="h-4 w-4" />
          <StopIcon v-else class="h-4 w-4" />
        </button>
      </span>
      <!-- Show in terminal -->
      <span
        v-if="expanded"
        class="absolute -bottom-5 right-0 flex animate-fadein-500 flex-row items-center gap-0.5 text-xs text-gray-400 underline-offset-2 transition-opacity duration-150 hover:cursor-pointer hover:underline"
        @click="bench.openTerminal({ group: panel.panel.value.group, focus: true, opposite: true })"
      >
        <CommandLineIcon class="h-4 w-4" /> Terminal
      </span>
    </div>
    <!-- Generated -->
    <FadeTransition>
      <div
        v-if="generatedCode != null"
        class="relative flex w-[600px] flex-col gap-1 rounded-sm bg-white px-3 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :style="{ width: fullWidth + 'px' }"
      >
        <!-- Header -->
        <div class="flex flex-row gap-1 px-1">
          <SparklesIcon class="h-4 w-4 text-orange-600" />
          <span class="max-w-full truncate font-semibold">{{ generatedFrom }}</span>
          <!-- TODO @UX: jump to terminal from terminal popover generation -->
          <!-- <span
            class="ml-auto flex flex-row items-center gap-1 text-sm text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
            @click="bench.openTerminal({ group: panel.panel.value.group, focus: true, opposite: true })"
          >
            <CommandLineIcon class="h-4 w-4" /> To Terminal
          </span> -->
          <span
            v-if="generatedRun != null"
            class="ml-auto text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
            @click="bench.openViewRun(generatedRun, { group: panel.panel.value.group, opposite: true, focus: true })"
          >
            #{{ getUUIDFromGlobalID(generatedRun.id).slice(-7, -1) }}
          </span>
        </div>
        <!-- Code preview -->
        <MonacoEditor
          v-model="generatedCode"
          class="max-h-80 overflow-y-auto"
          language="python"
          :focused="active"
          :wrap="generatedCodeCondensed"
        />
        <!-- Controls -->
        <!-- Apply/Discard should also tag the task run with feedback (as a demo and because it would be useful) -->
        <div
          class="right-2 flex flex-row-reverse gap-3 rounded-sm"
          :class="generatedCodeCondensed ? '' : '  absolute bottom-2 bg-white/80'"
        >
          <!-- Apply -->
          <button
            class="flex flex-row items-center gap-1 rounded-sm bg-orange-600 px-1.5 py-1 text-white hover:bg-orange-500"
            @click="apply"
          >
            <PlayIcon class="h-4 w-4" />
            <span>Apply</span>
          </button>
          <!-- Reject -->
          <button
            class="flex flex-row items-center gap-1 rounded-sm px-1.5 py-1 text-gray-700 hover:bg-orange-100"
            @click="discardGenerated"
          >
            <XMarkIcon class="h-4 w-4" />
            <span>Discard</span>
          </button>
        </div>
      </div>
    </FadeTransition>
  </div>
</template>
