<script lang="ts" setup>
import loader, { type Monaco } from "@monaco-editor/loader";
import type * as monaco from "monaco-editor";
import { onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
  language: "json" | "python";
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();
let editor: Ref<monaco.editor.IStandaloneCodeEditor | null> = ref(null);

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

const editorContainer: Ref<HTMLElement | null> = ref(null);
onMounted(() => {
  loader.init().then(initMonaco);
});

function initMonaco(monaco: Monaco) {
  if (!editorContainer.value) {
    throw new Error("editor container not found");
  }

  // define custom theme
  monaco.editor.defineTheme("bench", {
    base: "vs",
    inherit: true,
    rules: [],
    colors: {
      // set line numbers color
      "editorLineNumber.foreground": "#fdba74",
      "editorLineNumber.activeForeground": "#f97316",
      // set selection color
      "editor.selectionBackground": "#fdba74",
      // set cursor color
      "editorCursor.foreground": "#f97316",
      // hide line
      "editor.lineHighlightBackground": "#ffffff",
    },
  });

  // set height based on line count
  editorContainer.value.style.height = getEditorHeight(props.modelValue) + "px";

  // create editor
  editor.value = monaco.editor.create(editorContainer.value, {
    value: props.modelValue,
    language: props.language,
    minimap: {
      enabled: false,
    },
    scrollBeyondLastLine: false,
    readOnly: true,
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
    console.log("update model value from editor");
    // TODO @Feature: update model value
    //  (for some reason monaco crashes when calling editor.getValue() here)
    emit("update:modelValue", props.modelValue);
  });
}

// close monaco editor on unmount
// TODO @Cleanup: properly dispose monaco editor on unmount
//  (just calling .dispose like below hangs the app)
onBeforeUnmount(() => {
  if (editor.value) {
    // editor.value.dispose();
  }
});
</script>

<template>
  <div class="w-full" id="editor-container">
    <div ref="editorContainer" class="editor-root h-full w-full"></div>
  </div>
</template>
