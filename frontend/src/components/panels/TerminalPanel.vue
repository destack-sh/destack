<script lang="ts" setup>
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, TerminalPanel } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { SESSION_ACCESS_LEVELS, useTerminal } from "@/state/session";
import { syncProperty } from "@/utils/sync";
import { ChevronDoubleRightIcon, ChevronRightIcon } from "@heroicons/vue/24/outline";
import { ArrowRightIcon, PlayIcon } from "@heroicons/vue/24/solid";
import { Bars3BottomLeftIcon, CodeBracketIcon } from "@heroicons/vue/24/solid";
import { computed, ref, type Ref } from "vue";

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
syncProperty({
  read: () => (input.value = panel.value.input ?? ""),
  write: () => (panel.value.input = input.value),
  debounceMs: 500,
});
const inputRef: Ref<InstanceType<typeof MonacoEditor | typeof AnnotatedText> | null> = ref(null);

function focus(f: "first" | "last" = "last") {
  inputRef.value?.focus?.("last");
}

function run() {
  if (panel.value.inputMode == "code") {
    terminal.runCode(input.value);
  } else {
    terminal.runText(input.value);
  }
}

defineExpose({
  focus,
});
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }" @click="focus()">
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
      class="mx-auto flex w-full max-w-full flex-1 flex-col pt-8 text-sm"
      :style="{ ...panel.contentWidthAsFixed, ...panel.contentMarginXAsPaddingX }"
    >
      <!-- nocheckin: todo terminal history -->
      yo {{ terminal.runs.value.length }}
    </div>
    <!-- Input (bottom)-->
    <div class="w-full border-t border-orange-900/[15%] bg-white">
      <div
        class="mx-auto flex w-full max-w-full flex-row gap-x-2 bg-white pb-3.5 pl-3 pr-4 pt-3 text-sm text-gray-900"
        :style="panel.contentWidthAsFixed"
      >
        <div class="flex w-full flex-1 flex-col">
          <!-- Header -->
          <div class="flex flex-row items-start">
            <span class="px-1">
              <ChevronDoubleRightIcon class="h-4 w-4 text-orange-600" />
            </span>
            <!-- Current context/path & mode -->
            <span class="ml-0.5 font-semibold text-orange-600">{{ module.path.value }}</span>
            <!-- Access level -->
            <button
              class="hover ml-2 rounded-sm bg-emerald-100 px-1.5 text-emerald-900 ring-1 ring-inset ring-emerald-600/20 hover:bg-emerald-200"
              @click="
                () => {
                  const levels = SESSION_ACCESS_LEVELS;
                  panel.accessLevel = levels[(levels.indexOf(panel.accessLevel) + 1) % levels.length];
                }
              "
            >
              can {{ panel.accessLevel.toLowerCase() }}
            </button>
          </div>
          <!-- Body -->
          <div class="flex flex-row">
            <!-- Text/Code toggle -->
            <button
              @click="panel.toggleInputMode()"
              class="flex h-fit flex-row rounded-sm px-1 py-[1px] text-orange-600 hover:bg-orange-100"
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
                :focused="panel.focused"
                language="python"
                hide-line-numbers
                @click.stop
                @execute="run"
              />
              <!-- Placeholder -->
              <span v-if="input.length == 0" class="absolute left-0 top-0 text-gray-400">
                Enter {{ panel.inputMode == "code" ? "code" : "text" }}...
              </span>
            </div>
          </div>
        </div>
        <!-- Run -->
        <button class="rounded-sm px-1.5 py-1 hover:bg-orange-100" @click="run">
          <PlayIcon class="h-6 w-6 text-orange-600" />
        </button>
      </div>
    </div>
  </div>
</template>
