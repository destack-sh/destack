<script lang="ts" setup>
import loader, { type Monaco } from "@monaco-editor/loader";
import { useElementSize } from "@vueuse/core";
import type * as monaco from "monaco-editor";
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
  language: "json" | "python";
  focused: boolean;
  readonly?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

let editor: Ref<monaco.editor.IStandaloneCodeEditor | null> = shallowRef(null);

// sync modelValue into editor
watch(
  () => props.modelValue,
  (value) => {
    if (editor.value && value !== editor.value.getValue()) {
      console.log("update editor from model value");
      editor.value.setValue(value);
    }
  }
);

function getEditorHeight(code: string) {
  const lines = code.split("\n").length;
  return lines * 19;
}

function updateEditorHeight(container: HTMLElement, code: string) {
  container.style.height = getEditorHeight(code) + "px";
  editor.value?.layout({ width: container.clientWidth, height: container.clientHeight });
}

function onResize() {
  if (editor.value && editorContainer.value) {
    updateEditorHeight(editorContainer.value, props.modelValue);
  }
}

const editorContainer: Ref<HTMLElement | null> = ref(null);
const { width: editorContainerWidth } = useElementSize(editorContainer);
// resize editor when container width changes
watch(editorContainerWidth, onResize);

onMounted(() => {
  loader.init().then(initMonaco);
});

const BENCH_THEME_COLORS = {
  // transparent background
  "editor.background": "#00000000",
  // line numbers (when not focused, focused colors are set in style bellow)
  "editorLineNumber.foreground": "#fed7aa",
  "editorLineNumber.activeForeground": "#fdba74",
  // set selection color
  "editor.selectionBackground": "#fdba74",
  // set cursor color
  "editorCursor.foreground": "#f97316",
  // hide line
  "editor.lineHighlightBackground": "#ffffff",
  // set color for matching brackets
  "editorBracketMatch.background": "#ea580c",
  "editorBracketMatch.border": "#ea580c",
};

function initMonaco(monaco: Monaco) {
  if (!editorContainer.value) {
    // can happen when component is unmounted again before initMonaco is called
    console.error("editor container not found");
    return;
  }
  if (editor.value !== null) {
    throw new Error("editor already initialized");
  }

  // define custom theme
  monaco.editor.defineTheme("bench", {
    base: "vs",
    inherit: true,
    rules: [
      // make keywords orange
      { token: "keyword", foreground: "#ea580c" },
      { token: "string.key.json", foreground: "#ea580c" },
      // make comments grey
      { token: "comment", foreground: "#6b7280" },
      // make literals and constants orange
      { token: "number", foreground: "#d97706" },
      { token: "string", foreground: "#d97706" },
      { token: "string.value.json", foreground: "#d97706" },
      { token: "constant.numeric", foreground: "#d97706" },
      { token: "constant.character", foreground: "#d97706" },
      { token: "constant.language", foreground: "#d97706" },
      { token: "constant.other", foreground: "#d97706" },
      // TODO @UX: make builtin functions orange and bold
      // (the below doesn't work because the token type isn't defined yet)
      { token: "bench-builtin-function", foreground: "#d97706", fontStyle: "bold" },
    ],
    colors: BENCH_THEME_COLORS,
  });

  // set height based on line count
  updateEditorHeight(editorContainer.value, props.modelValue);

  // create editor
  editor.value = monaco.editor.create(editorContainer.value, {
    value: props.modelValue,
    language: props.language,
    minimap: {
      enabled: false,
    },
    readOnly: props.readonly,
    scrollBeyondLastLine: false,
    lineDecorationsWidth: 24,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    overviewRulerLanes: 0,
    lineNumbersMinChars: 3,
    lineNumbers: "on",
    renderLineHighlight: "none",
    // disable folding
    folding: false,
    scrollbar: {
      vertical: "hidden",
      horizontal: "hidden",
      handleMouseWheel: false,
    },
    theme: "bench",
  });

  editor.value.onDidChangeModelContent(() => {
    const value = editor.value?.getValue();
    if (value && editorContainer.value) {
      updateEditorHeight(editorContainer.value, value);
      emit("update:modelValue", value);
    }
  });
}

// close monaco editor on unmount
onBeforeUnmount(() => {
  if (editor.value) {
    // close in next tick to avoid slow frame
    nextTick(() => editor.value?.dispose());
  }
});
</script>

<template>
  <div class="w-full">
    <div ref="editorContainer" :class="props.focused ? 'focused' : ''" class="editor-root h-full w-full" />
  </div>
</template>

<style>
.focused .line-numbers {
  color: #fdba74 !important;
}
.focused .active-line-number {
  color: #ea580c !important;
  font-weight: 600;
}
</style>
