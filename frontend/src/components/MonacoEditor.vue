<script lang="ts" setup>
import loader, { type Monaco } from "@monaco-editor/loader";
import type * as monaco from "monaco-editor";
import { onBeforeUnmount, onMounted, ref, shallowRef, toRef, watch, type Ref } from "vue";

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
}

const editorContainer: Ref<HTMLElement | null> = ref(null);
onMounted(() => {
  loader.init().then(initMonaco);
});

const BENCH_THEME_COLORS = {
  "editorLineNumber.foreground": "#fed7aa",
  "editorLineNumber.activeForeground": "#fb923c",
  // set selection color
  "editor.selectionBackground": "#fdba74",
  // set cursor color
  "editorCursor.foreground": "#f97316",
  // hide line
  "editor.lineHighlightBackground": "#ffffff",
};

function initMonaco(monaco: Monaco) {
  if (!editorContainer.value) {
    throw new Error("editor container not found");
  }

  // define custom theme
  monaco.editor.defineTheme("bench", {
    base: "vs",
    inherit: true,
    rules: [],
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
    lineDecorationsWidth: 10,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    overviewRulerLanes: 0,
    lineNumbersMinChars: 3,
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

  editor.value.onDidChangeModelContent((event) => {
    console.log("update model value from editor", event);
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
    editor.value.dispose();
  }
});
</script>

<template>
  <div class="w-full" id="editor-container">
    <div ref="editorContainer" :class="props.focused ? 'focused' : ''" class="editor-root h-full w-full"></div>
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
