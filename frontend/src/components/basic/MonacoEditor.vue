<script lang="ts" setup>
import loader, { type Monaco } from "@monaco-editor/loader";
import { useElementSize } from "@vueuse/core";
import type * as monaco from "monaco-editor";
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
  language: "json" | "jsonl" | "csv" | "python" | "markdown" | "btl";
  focused: boolean;
  readonly?: boolean;
  lineNumberOffset: number;
  lineNumberShiftPx?: number;
  hideLineNumbers?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "deleteIfEmpty"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "escape"): void;
  (e: "enter"): void;
  (e: "toggleActions"): void;
  (e: "execute"): void;
}>();

const editor: Ref<monaco.editor.IStandaloneCodeEditor | null> = shallowRef(null);
const focused: Ref<boolean> = ref(false);

function getEditorHeight(code: string) {
  let lines = code.split("\n").length;
  if (lines == 0) lines = 1;
  return lines * 21; // TODO @Robustness: compute monaco line height automatically
}

function updateEditorHeight(container: HTMLElement, code: string) {
  container.style.height = getEditorHeight(code) + "px";
  editor.value?.layout({ width: container.clientWidth, height: container.clientHeight });
}

function onResize() {
  if (editorContainer.value) {
    updateEditorHeight(editorContainer.value, props.modelValue);
  }
}

const editorContainer: Ref<HTMLElement | null> = ref(null);
const { width: editorContainerWidth } = useElementSize(editorContainer);
// resize editor when container width changes
watch(editorContainerWidth, onResize, { immediate: true });

onMounted(() => {
  loader.init().then(initMonaco);
});

const BENCH_THEME_COLORS = {
  // transparent background
  "editor.background": "#00000000",
  // line numbers (when not focused, focused colors are set in style bellow)
  "editorLineNumber.foreground": "#d4d4d8",
  "editorLineNumber.activeForeground": "#d4d4d8",
  // set selection color (sync with ::selection in App.vue)
  "editor.selectionBackground": "#fef08a",
  // set cursorcolor
  "editorCursor.foreground": "#f97316",
  // hide line
  "editor.lineHighlightBackground": "#ffffff",
  // set color for matching brackets
  "editorBracketMatch.background": "#fef08a",
  "editorBracketMatch.border": "#fef08a",
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
      { token: "keyword", foreground: "#b45309" },
      { token: "string.key.json", foreground: "#b45309" },
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
      { token: "identifier.python", foreground: "#000000" },
      // make comments italic
      { token: "comment", fontStyle: "italic" },
      // (the below doesn't work because the token type isn't defined yet)
      { token: "bench-builtin-function", foreground: "#d97706", fontStyle: "bold" },
    ],
    colors: BENCH_THEME_COLORS,
  });

  // set height based on line count
  updateEditorHeight(editorContainer.value, props.modelValue);

  // create editor
  const lineNumbers = (i: number) => (i + (props.lineNumberOffset ?? 0)).toString();
  editor.value = monaco.editor.create(editorContainer.value, {
    value: props.modelValue,
    language: props.language,
    minimap: {
      enabled: false,
    },
    readOnly: props.readonly,
    scrollBeyondLastLine: false,
    lineDecorationsWidth: 8,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    overviewRulerLanes: 0,
    lineNumbersMinChars: 2,
    // set font to same mono from tailwind config
    fontSize: 14,
    fontFamily: "Druid Sans Mono, monospace",
    lineNumbers: props.hideLineNumbers ? "off" : lineNumbers,
    renderLineHighlight: "none",
    scrollbar: {
      vertical: "hidden",
      horizontal: "hidden",
      handleMouseWheel: false,
    },
    // disable indent highlighting
    guides: {
      indentation: false,
      highlightActiveIndentation: false,
    },
    theme: "bench",
    occurrencesHighlight: false, // should be yellow but can't figure out how
    contextmenu: false,
  });

  editor.value.onDidChangeModelContent(() => {
    const value = editor.value?.getValue();
    if (value != null && editorContainer.value != null) {
      updateEditorHeight(editorContainer.value, value);
      emit("update:modelValue", value);
    }
  });

  function handleCommands() {
    if (editor.value == null) return;

    // handle key events (delete if empty, navigate up/down if top/bottom, etc.)
    editor.value.addCommand(monaco.KeyMod.Shift | monaco.KeyCode.Enter, () => emit("enter"));
    editor.value.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => emit("execute"));
    editor.value.addCommand(monaco.KeyMod.WinCtrl | monaco.KeyCode.Enter, () => emit("execute"));
    editor.value.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.Enter, () => emit("toggleActions"));
    editor.value.onKeyDown((e) => {
      if (e.keyCode === monaco.KeyCode.Backspace) {
        if (editor.value?.getValue() === "") {
          emit("deleteIfEmpty");
        }
      } else if (e.keyCode === monaco.KeyCode.UpArrow) {
        if (!e.shiftKey && !e.altKey && editor.value?.getPosition()?.lineNumber === 1) {
          emit("navigateUp");
        }
      } else if (e.keyCode === monaco.KeyCode.DownArrow) {
        if (
          !e.shiftKey &&
          !e.altKey &&
          editor.value?.getPosition()?.lineNumber === editor.value?.getModel()?.getLineCount()
        ) {
          emit("navigateDown");
        }
      } else if (e.keyCode == monaco.KeyCode.Escape) {
        // trigger outer escape if no widget is open and visible
        const widgetClasses = [".monaco-menu-container", ".monaco-dropdown", ".monaco-editor-hover", ".editor-widget"];
        const openWidget = widgetClasses.find((c) => {
          const els = document.querySelectorAll(c);
          for (let i = 0; i < els.length; i++) {
            const el = els[i] as HTMLElement;
            if (el.style.display != "none" && el.getAttribute("aria-hidden") != "true") {
              return true;
            }
          }
        });
        if (!openWidget) {
          emit("escape");
          (document.activeElement as HTMLElement)?.blur?.();
        }
      }
    });
  }

  // update focused when editor is focused/defocused
  editor.value.onDidFocusEditorWidget(() => {
    handleCommands(); // always re-register to ensure this runs last
    // see https://github.com/microsoft/monaco-editor/issues/2947
    focused.value = true;
  });
  editor.value.onDidBlurEditorWidget(() => {
    focused.value = false;
  });
}

// sync modelValue into editor
watch(
  () => props.modelValue,
  (value) => {
    if (editor.value && value !== editor.value.getValue()) {
      editor.value.setValue(value);
    }
  }
);
// sync line number offset into editor
watch(
  () => [props.lineNumberOffset, props.hideLineNumbers],
  () => {
    if (editor.value) {
      if (!props.hideLineNumbers) {
        const lineNumbers = (i: number) => (i + (props.lineNumberOffset ?? 0)).toString();
        editor.value.updateOptions({ lineNumbers });
      } else {
        editor.value.updateOptions({ lineNumbers: "off" });
      }
    }
  }
);
// sync line number offset into editor (as line decorations witdh)
watch(
  () => props.lineNumberShiftPx,
  () => {
    if (editor.value) {
      editor.value.updateOptions({ lineDecorationsWidth: props.lineNumberShiftPx ?? 24 });
    }
  }
);

// close monaco editor on unmount
onBeforeUnmount(() => {
  if (editor.value) {
    // close in next tick to avoid slow frame
    nextTick(() => editor.value?.dispose());
  }
});

function focus(end?: boolean) {
  editor.value?.focus();
  if (end) {
    editor.value?.setPosition({
      lineNumber: editor.value?.getModel()?.getLineCount() ?? 1,
      column: editor.value?.getModel()?.getLineMaxColumn(editor.value?.getModel()?.getLineCount() ?? 1) ?? 0,
    });
  }
}
function blur() {
  // no op?
}

defineExpose({ focus, blur, focused });
</script>

<template>
  <div class="w-full">
    <div ref="editorContainer" :class="props.focused ? 'focused' : ''" class="editor-root h-full w-full" />
  </div>
</template>

<style>
.line-numbers {
  user-select: none;
}

.focused .line-numbers {
  color: #fdba74 !important;
}
.focused .active-line-number {
  color: #ea580c !important;
  font-weight: 600;
}

/* disable blue square dot from tailwind */
.monaco-editor textarea:focus {
  box-shadow: none !important;
}
</style>
