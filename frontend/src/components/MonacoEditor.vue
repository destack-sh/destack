<script lang="ts" setup>
import loader from "@monaco-editor/loader";
import type * as monaco from "monaco-editor";
import { onMounted, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
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

onMounted(() => {
  loader.init().then((monaco) => {
    const editorContainer = document.getElementById("editor-container");
    if (!editorContainer) {
      throw new Error("div with id 'editor-container' not found");
    }
    editorContainer.style.height = getEditorHeight(props.modelValue) + "px";

    const editorDiv = document.getElementById("editor");
    if (!editorDiv) {
      throw new Error("div with id 'editor' not found");
    }
    editor.value = monaco.editor.create(editorDiv, {
      value: props.modelValue,
      language: "python",
      minimap: {
        enabled: false,
      },
      scrollBeyondLastLine: false,
      readOnly: true,
      lineDecorationsWidth: 0,
      hideCursorInOverviewRuler: true,
      overviewRulerBorder: false,
      overviewRulerLanes: 0,
      lineNumbersMinChars: 2,
      scrollbar: {
        vertical: "hidden",
        horizontal: "hidden",
        handleMouseWheel: false,
      },
    });
    editor.value.onDidChangeModelContent((event) => {
      console.log("update model value from editor");
      // TODO @Feature: update model value
      //  (for some reason monaco crashes when calling editor.getValue() here)
      emit("update:modelValue", props.modelValue);
    });
  });
});
</script>

<template>
  <div class="w-full" id="editor-container">
    <div id="editor" class="h-full w-full"></div>
  </div>
</template>
