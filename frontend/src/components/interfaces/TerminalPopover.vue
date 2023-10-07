<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { useBenchState, type StatementHeader, usePanelContext } from "@/state/bench";
import { useTerminal } from "@/state/terminal";
import { makeTextMention, makeTextPlain, renderTextHtml, type TextSpan } from "@/state/text";
import { CommandLineIcon } from "@heroicons/vue/24/outline";
import { PlayIcon, SparklesIcon } from "@heroicons/vue/24/solid";
import { onClickOutside } from "@vueuse/core";
import { ref, nextTick, computed } from "vue";

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

onClickOutside(containerRef, () => {
  if (active.value) {
    close();
  }
});

function run() {
  console.log("nocheckin run", inputText.value);
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
    :class="[active ? '' : 'opacity-80 focus-within:opacity-100 hover:opacity-100']"
  >
    <!-- Header/input -->
    <div
      class="relative flex flex-row gap-1 bg-white px-2 py-1.5 shadow-md ring-1 ring-orange-900 transition-all duration-150"
      :class="[
        active
          ? 'w-[425px] rounded-sm ring-opacity-40'
          : 'w-10 rounded-2xl ring-opacity-20 hover:cursor-pointer hover:bg-orange-100',
      ]"
      @click="active ? focus() : open(inputText)"
      @keydown.escape.stop.prevent="close()"
    >
      <!-- Open/close button -->
      <button
        class="flex-shrink-0 self-start rounded-sm p-1 text-orange-600 transition-colors duration-150 hover:cursor-pointer hover:bg-orange-100"
        @click="(e) => !active || (e.stopPropagation(), close())"
      >
        <SparklesIcon class="h-4 w-4" />
      </button>
      <!-- Actual input -->
      <AnnotatedText
        v-if="active"
        ref="inputRef"
        class="min-w-[5px] max-w-full overflow-x-hidden whitespace-normal py-0.5"
        v-model="inputText"
        suppress-shortcuts
        @click.stop
        @enter="run"
        @enter-right="run"
      />
      <span v-if="inputText.length == 0">&nbsp;</span>
      <!-- Run -->
      <button
        v-if="active"
        class="ml-auto flex-shrink-0 self-start rounded-sm p-1 transition-colors duration-150 hover:bg-orange-100"
        :class="[canRun ? 'text-orange-600' : 'text-gray-400']"
        :disabled="!canRun"
        @click="run()"
      >
        <PlayIcon class="h-4 w-4" />
      </button>
      <!-- Show in terminal -->
      <span
        v-if="active"
        class="absolute -bottom-5 right-0 flex flex-row items-center gap-0.5 text-xs text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
        @click="bench.openTerminal({ group: panel.panel.value.group, focus: true, opposite: true })"
      >
        <CommandLineIcon class="h-4 w-4" /> Terminal
      </span>
    </div>
    <!-- nocheckin: terminal popover -->
    <!-- History? -->
  </div>
</template>
