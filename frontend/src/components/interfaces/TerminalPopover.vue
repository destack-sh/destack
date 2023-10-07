<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { useBenchState, type StatementHeader, usePanelContext } from "@/state/bench";
import { SessionAccessLevel } from "@/state/session";
import { useTerminal } from "@/state/terminal";
import { makeTextMention, makeTextPlain, renderTextHtml, type TextSpan } from "@/state/text";
import { CommandLineIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { PlayIcon, SparklesIcon } from "@heroicons/vue/24/solid";
import { onClickOutside } from "@vueuse/core";
import { ref, nextTick, computed } from "vue";

const props = defineProps<{
  fileCk: string;
}>();
const bench = useBenchState();
const panel = usePanelContext();
const terminal = useTerminal();

const containerRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof AnnotatedText> | null>(null);
const inputText = ref("");
const canRun = computed(
  // check if there is any meaningful text
  () => inputRef.value?.spans.some((s) => s.type == "text" && s.text.trim() != "") ?? false
);
const active = ref(false);
const expanded = computed(() => active.value || generating.value || generatedCode.value != null);
const generating = ref(false);
const generatedFrom = ref<string | null>(null);
const generatedCode = ref<string | null>(null);
const applyingCode = ref(false);

onClickOutside(containerRef, () => {
  if (active.value) {
    inputText.value = "";
    close();
  }
});

async function run() {
  if (generating.value) return;
  discard();
  generating.value = true;
  try {
    generatedFrom.value = inputText.value;
    const { code } = await terminal.runText(inputText.value);
    generatedCode.value = code;
  } finally {
    generating.value = false;
  }
}

function discard() {
  generatedCode.value = null;
  generatedFrom.value = null;
}

async function apply() {
  if (generatedCode.value == null || generatedFrom.value == null) return;
  applyingCode.value = true;
  try {
    await terminal.runCode(generatedCode.value, { scope: props.fileCk, accessLevel: SessionAccessLevel.Update });
    discard();
    inputText.value = "";
    close();
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
          ? 'w-[600px] rounded-sm ring-opacity-40'
          : 'w-10 rounded-2xl ring-opacity-20 hover:cursor-pointer hover:bg-orange-100',
      ]"
      @click="active ? focus() : open(inputText)"
      @keydown.escape.stop.prevent="discard(), close()"
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
        @click.stop
        @enter="run"
        @enter-right="run"
      />
      <span v-if="inputText.length == 0">&nbsp;</span>
      <!-- Run -->
      <button
        v-if="expanded"
        class="ml-auto flex-shrink-0 self-start rounded-sm p-1 transition-colors duration-150 hover:bg-orange-100"
        :class="[canRun ? 'text-orange-600' : 'text-gray-400']"
        :disabled="!canRun || generating"
        @click="run()"
      >
        <PlayIcon v-if="!generating" class="h-4 w-4" />
        <BusySpinnerIcon v-else class="h-4 w-4 animate-spin" />
      </button>
      <!-- Show in terminal -->
      <span
        v-if="expanded"
        class="absolute -bottom-5 right-0 flex flex-row items-center gap-0.5 text-xs text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
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
      >
        <!-- Header -->
        <div class="flex flex-row gap-1 px-1">
          <SparklesIcon class="h-4 w-4 text-orange-600" />
          <span class="max-w-full truncate font-semibold">{{ generatedFrom }}</span>
          <!-- Jump to terminal -->
          <span
            class="ml-auto flex flex-row items-center gap-1 text-sm text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
            @click="bench.openTerminal({ group: panel.panel.value.group, focus: true, opposite: true })"
          >
            <CommandLineIcon class="h-4 w-4" /> To Terminal
          </span>
        </div>
        <!-- Code preview -->
        <MonacoEditor v-model="generatedCode" class="max-h-80 overflow-y-auto" language="python" :focused="active" />
        <!-- Controls -->
        <div class="absolute bottom-2 right-2 flex flex-row-reverse gap-3 rounded-sm bg-white/80">
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
            @click="discard"
          >
            <XMarkIcon class="h-4 w-4" />
            <span>Discard</span>
          </button>
        </div>
      </div>
    </FadeTransition>
  </div>
</template>
