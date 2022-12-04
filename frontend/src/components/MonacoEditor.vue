<script lang="ts" setup>
import loader, { type Monaco } from "@monaco-editor/loader";
import type * as monaco from "monaco-editor";
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Ref } from "vue";
import { useElementSize } from "@vueuse/core";

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

const numLines = computed(() => props.modelValue.split("\n").length);
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
  if (editor.value !== null) {
    throw new Error("editor already initialized");
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
    editor.value.dispose();
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
