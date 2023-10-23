<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import Switch from "@/components/basic/Switch.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { StatementType, type Run, RunStatus } from "@/gql/graphql";
import { useBenchState, type StatementHeader, usePanelContext } from "@/state/bench";
import { useNavigation, type NodeBase } from "@/state/module";
import { SessionAccessLevel, useCurrentSessions } from "@/state/session";
import { useTerminal } from "@/state/terminal";
import { makeTextMention, makeTextPlain, renderTextHtml, type TextSpan } from "@/state/text";
import { getUUIDFromGlobalID, useDelayed } from "@/utils/functools";
import { BoltIcon, CommandLineIcon, LightBulbIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { ArrowUturnLeftIcon, PlayIcon, SparklesIcon, StopIcon } from "@heroicons/vue/24/solid";
import { onClickOutside } from "@vueuse/core";
import { ref, nextTick, computed, type Ref } from "vue";

const props = defineProps<{
  fileCk: string;
  currentSelection?: StatementHeader[];
}>();
const bench = useBenchState();
const panel = usePanelContext();
const terminal = useTerminal();
const session = useCurrentSessions();
const nav = useNavigation();

const containerRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof AnnotatedText> | null>(null);
const inputFrom = ref<StatementHeader | null>(null);
const inputText = ref("");
const mode: Ref<"fast" | "deliberate"> = ref("fast");
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
const appliedFailedRun = ref<Run | null>(null);

const fullWidth = computed(() => Math.min(panel.size.value.width - 40, 600));

onClickOutside(containerRef, () => {
  if (active.value) {
    inputText.value = "";
    close(false);
  }
});

async function run(config?: { nonce?: string; mode?: "fast" | "deliberate" }) {
  if (!canRun.value) return;
  if (generatingRun.value) {
    cancel();
  }
  discardGenerated();
  try {
    // clear input text from span references (ignore content)
    generatedFrom.value = inputText.value.replace(/<span.*?>/g, "").replace(/<\/span>/g, "");
    const { result, run } = terminal.runTextToCode(inputText.value, { mode: mode.value, ...config });
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
  appliedFailedRun.value = null;
}

async function apply() {
  if (generatedCode.value == null || generatedFrom.value == null) return;
  applyingCode.value = true;
  appliedFailedRun.value = null;
  try {
    const appendAfter = inputFrom.value ? `module.resolve(UUID("${getUUIDFromGlobalID(inputFrom.value.id)}"))` : "None";
    const appendLine = `file.extend(session.dangling_like(Statement), after=${appendAfter}) # auto-generated`;
    const code = generatedCode.value + "\n" + appendLine;
    const { finalResult } = terminal.runCode(code, {
      scope: props.fileCk,
      accessLevel: SessionAccessLevel.Update,
      tags: ["mend"],
      generatedFrom: generatedFrom.value,
      generatedIn: generatedRun.value != null ? getUUIDFromGlobalID(generatedRun.value.id) : undefined,
    });
    const { run } = await finalResult;
    if (run.status != RunStatus.Completed) {
      appliedFailedRun.value = run;
      throw new Error(`run ${getUUIDFromGlobalID(run.id)} failed: ${run?.errorNice?.kind} ${run?.errorNice?.message}`);
    }
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
  inputFrom.value = from ?? null;
  if (!text && !selection) {
    // default to current selection if nothing is provided
    selection = props.currentSelection;
  }
  selection = selection?.filter((s) => s.type != StatementType.Blank); // blanks are unhelpful
  if (selection) {
    // render selection into text
    const spans: TextSpan[] = [...selection.map((s) => makeTextMention(s)), makeTextPlain(" " + text)];
    text = renderTextHtml(spans);
  }
  inputText.value = text ?? "";
  active.value = true;
  nextTick(focus);
}

function close(refocus = true) {
  active.value = false;
  if (refocus && inputFrom.value != null) {
    nav.focusStatement(inputFrom.value as NodeBase);
  }
}

function escape() {
  discardGenerated();
  close();
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
    class="flex flex-col-reverse gap-2 text-sm text-gray-900 transition-colors duration-150"
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
      @keydown.escape.stop.prevent="escape()"
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
        @execute="apply"
      />
      <span v-if="inputText.length == 0">&nbsp;</span>
      <!-- Run -->
      <span v-if="expanded" class="ml-auto flex flex-shrink-0 items-center self-start">
        <!-- Active -->
        <FadeTransition>
          <span v-if="generatingRun != null" class="mr-0.5 text-gray-400">
            {{ session.getDurationFormatted(generatingRun, { hideMillis: true }) }}
          </span>
        </FadeTransition>
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
        class="absolute -bottom-5 left-0 flex animate-fadein-500 flex-row items-center gap-0.5 text-xs text-gray-400 underline-offset-2 transition-opacity duration-500 hover:cursor-pointer hover:underline"
        @click="bench.openTerminal({ group: panel.panel.value.group, focus: true, opposite: true })"
      >
        <CommandLineIcon class="h-4 w-4" /> Terminal
      </span>
      <!-- Task mode -->
      <button
        v-if="expanded"
        class="absolute -bottom-5 right-0 flex animate-fadein-500 flex-row items-center gap-0.5 rounded-sm text-xs text-gray-400 transition-opacity duration-500 hover:bg-orange-100 hover:text-gray-700"
        @click="mode = mode == 'fast' ? 'deliberate' : 'fast'"
      >
        <component :is="mode == 'fast' ? BoltIcon : LightBulbIcon" class="h-4 w-4" />
        {{ mode == "fast" ? "Fast" : "Deliberate" }}
      </button>
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
          <span
            v-if="generatedRun != null"
            class="ml-auto text-gray-400 underline-offset-2 hover:cursor-pointer hover:underline"
            @click.stop="
              bench.openViewRun(generatedRun, { group: panel.panel.value.group, opposite: true, focus: true })
            "
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
          @execute="apply"
        />
        <!-- Error -->
        <div v-if="appliedFailedRun != null" class="my-0.5 flex w-full flex-row bg-red-100 px-2 py-1">
          <span class="font-mono text-red-600">
            Failed to apply:
            {{ appliedFailedRun.errorNice?.kind }}<template v-if="appliedFailedRun.errorNice?.message">:</template>
            {{ appliedFailedRun.errorNice?.message }}
          </span>
          <div class="ml-auto inline-block">
            <a
              class="text-gray-400 underline-offset-2 hover:underline"
              @click="
                bench.openViewRun(appliedFailedRun, { group: panel.panel.value.group, opposite: true, focus: true })
              "
              >#{{ getUUIDFromGlobalID(appliedFailedRun.id).slice(-7, -1) }}</a
            >
          </div>
        </div>
        <!-- Controls -->
        <!-- Apply/Discard should also tag the task run with feedback (as a demo and because it would be useful) -->
        <div
          class="right-2 flex flex-row-reverse gap-3 rounded-sm"
          :class="generatedCodeCondensed || appliedFailedRun ? '' : '  absolute bottom-2 bg-white/80'"
        >
          <!-- Apply -->
          <button
            class="flex flex-row items-center gap-1 rounded-sm bg-orange-600 px-1.5 py-1 text-white hover:bg-orange-500"
            @click="apply"
          >
            <PlayIcon v-if="!applyingCode" class="h-4 w-4" />
            <BusySpinnerIcon v-else class="h-4 w-4 animate-spin" />
            <span>Apply</span>
          </button>
          <!-- Retry -->
          <button
            class="flex flex-row items-center gap-1 rounded-sm border border-gray-300 px-1.5 py-1 text-gray-700 hover:bg-orange-100"
            @click="discardGenerated(), run({ mode: 'deliberate', nonce: Math.random().toString(36).slice(-8) })"
          >
            <ArrowUturnLeftIcon class="h-4 w-4" />
            <span>Retry</span>
          </button>
          <!-- Reject -->
          <button
            class="flex flex-row items-center gap-1 rounded-sm border border-gray-300 px-1.5 py-1 text-gray-700 hover:bg-orange-100"
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
