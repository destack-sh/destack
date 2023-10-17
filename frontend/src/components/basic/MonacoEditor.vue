<script lang="ts" setup>
import { cyrb53a } from "@/utils/functools";
import { INIT_MONACO } from "@/utils/globals";
import loader, { type Monaco } from "@monaco-editor/loader";
import { useElementSize } from "@vueuse/core";
import type * as monaco from "monaco-editor";
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
  language: "json" | "jsonl" | "csv" | "python" | "markdown" | "btl";
  focused: boolean;
  readonly?: boolean;
  hideLineNumbers?: boolean;
  enterIsExecute?: boolean;
  wrap?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "deleteIfEmpty"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "escape"): void;
  (e: "enter"): void;
  (e: "openActions"): void;
  (e: "execute"): void;
  (e: "toggleLanguage"): void;
}>();

const editor: Ref<monaco.editor.IStandaloneCodeEditor | null> = shallowRef(null);
const innerFocused: Ref<boolean> = ref(false);

function getEditorHeight(code: string) {
  // TODO @UX: calculate monaco editor height with proper line height, word wrapping, etc.
  if (editorContainer.value == null || editor.value == null) return 0;
  const lineHeight = 21;
  const lines = code.split("\n");
  let numLines = lines.length;

  if (props.wrap) {
    const maxCharsPerLine = Math.floor((editorContainer.value.clientWidth - 35) / 8.5);
    for (const line of lines) {
      const actualLines = Math.ceil(line.length / maxCharsPerLine);
      if (actualLines > 1) {
        numLines += actualLines - 1;
      }
    }
  }

  return numLines * lineHeight;
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
  // init monaco if not already initialized
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
    console.debug("editor container not found");
    return;
  }
  if (editor.value !== null) {
    throw new Error("editor already initialized");
  }

  // define custom theme
  if (!INIT_MONACO.value) {
    monaco.editor.defineTheme("bench", {
      base: "vs",
      inherit: true,
      rules: [
        // make keywords orange
        { token: "keyword", foreground: "#b45309" },
        { token: "string.key.json", foreground: "#b45309" },
        // make comments light grey
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
        // make brackets dark grey
        { token: "delimiter.bracket", foreground: "#6b7280" },
        { token: "delimiter.parenthesis", foreground: "#6b7280" },
      ],
      colors: BENCH_THEME_COLORS,
    });
    INIT_MONACO.value = true;
  }

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
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    overviewRulerLanes: 0,
    wordWrap: props.wrap ? "on" : "off",
    lineNumbers: props.hideLineNumbers ? "off" : "on",
    lineDecorationsWidth: props.hideLineNumbers ? 0 : 8,
    lineNumbersMinChars: props.hideLineNumbers ? 0 : 2,
    folding: false,
    // set font to same mono from tailwind config
    fontSize: 14,
    fontFamily: "Druid Sans Mono, monospace",
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
    bracketPairColorization: {
      enabled: false,
    },
    matchBrackets: "never",
  });

  function hasInnerWindowOpen() {
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
    return openWidget != null;
  }

  const registeredCommands = ref(false);
  function handleCommands() {
    if (editor.value == null) return;

    // handle key events (delete if empty, navigate up/down if top/bottom, etc.)
    editor.value.addAction({
      id: "execute",
      label: "Execute",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, monaco.KeyMod.WinCtrl | monaco.KeyCode.Enter],
      run: () => emit("execute"),
    });

    editor.value.addAction({
      id: "openActions",
      label: "Open Actions",
      keybindings: [monaco.KeyMod.Alt | monaco.KeyCode.Enter],
      run: () => emit("openActions"),
    });

    editor.value.addAction({
      id: "toggleLanguage",
      label: "Toggle Language",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Space, monaco.KeyMod.Alt | monaco.KeyCode.Space],
      run: () => emit("toggleLanguage"),
    });

    if (!props.enterIsExecute) {
      editor.value.addAction({
        id: "enter",
        label: "Enter",
        keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.Enter],
        run: () => {
          if (props.enterIsExecute) return;
          emit("enter");
        },
      });
    }

    if (!registeredCommands.value) {
      editor.value.onKeyDown((e) => {
        // only if we're focused
        if (!innerFocused.value) {
          return;
        }
        if (e.keyCode === monaco.KeyCode.Backspace) {
          if (editor.value?.getValue() === "") {
            emit("deleteIfEmpty");
          }
        } else if (e.keyCode === monaco.KeyCode.UpArrow) {
          if (!e.shiftKey && !e.altKey && editor.value?.getPosition()?.lineNumber === 1 && !hasInnerWindowOpen()) {
            e.stopPropagation();
            e.preventDefault();
            emit("navigateUp");
          }
        } else if (e.keyCode === monaco.KeyCode.DownArrow) {
          if (
            !e.shiftKey &&
            !e.altKey &&
            editor.value?.getPosition()?.lineNumber === editor.value?.getModel()?.getLineCount() &&
            !hasInnerWindowOpen()
          ) {
            e.stopPropagation();
            e.preventDefault();
            emit("navigateDown");
          }
        } else if (e.keyCode == monaco.KeyCode.Escape) {
          // trigger outer escape if no widget is open and visible
          if (!hasInnerWindowOpen()) {
            e.stopPropagation();
            e.preventDefault();
            emit("escape");
            (document.activeElement as HTMLElement)?.blur?.();
          }
        } else if (e.keyCode == monaco.KeyCode.Enter && !e.shiftKey) {
          if (props.enterIsExecute && !hasInnerWindowOpen()) {
            e.stopPropagation();
            e.preventDefault();
            emit("execute");
          }
        }
      });
    }
    editor.value.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      // suppress default save to file
    });

    registeredCommands.value = true;
  }

  // update focused when editor is focused/defocused
  editor.value.onDidFocusEditorWidget(() => {
    handleCommands(); // always re-register to ensure this runs last
    // see https://github.com/microsoft/monaco-editor/issues/2947
    // (not sure if this is still needed with addAction, but it's fiddly so let's keep it for now)
    innerFocused.value = true;
  });
  editor.value.onDidBlurEditorWidget(() => {
    innerFocused.value = false;
  });

  // sync model content change from editor
  editor.value.onDidChangeModelContent(() => {
    if (justSynced.value) {
      justSynced.value = false;
      return;
    }
    const value = editor.value?.getValue();
    if (value != null && editorContainer.value != null) {
      updateEditorHeight(editorContainer.value, value);
      emit("update:modelValue", value);
    }
  });
}

// sync modelValue into editor
const justSynced = ref(false);
watch(
  () => props.modelValue,
  (value) => {
    if (editor.value && value !== editor.value.getValue()) {
      // retain previous cursor position if value is in editor history
      justSynced.value = true;
      editor.value.setValue(value);
      restorePosition(value);
      updateEditorHeight(editorContainer.value!, value);
    }
  }
);

// remember view history for each 'significant' model value
// (we don't debounce or such in here, so the owner of this component must call 'mark' whenever a write happens so we can restore the view later)
const viewHistory: Record<number, monaco.Position | null> = {};
function markPosition() {
  // store current view with model value hash
  if (editor.value == null) return;
  const stateKey = cyrb53a(props.modelValue);
  viewHistory[stateKey] = editor.value.getPosition();
}
function restorePosition(value: string) {
  // restore view if model value is in history
  if (editor.value == null) return;
  const stateKey = cyrb53a(value);
  const pos = viewHistory[stateKey];
  if (pos != null) {
    editor.value.setPosition(pos);
  } else {
    editor.value?.setPosition({
      lineNumber: editor.value?.getModel()?.getLineCount() ?? 1,
      column: editor.value?.getModel()?.getLineMaxColumn(editor.value?.getModel()?.getLineCount() ?? 1) ?? 0,
    });
  }
}

// close monaco editor on unmount
onBeforeUnmount(() => {
  if (editor.value) {
    // close in next tick to avoid slow frame
    nextTick(() => editor.value?.dispose());
  }
});

function focus(f: "first" | "last" = "first") {
  editor.value?.focus();
  if (f == "last") {
    editor.value?.setPosition({
      lineNumber: editor.value?.getModel()?.getLineCount() ?? 1,
      column: editor.value?.getModel()?.getLineMaxColumn(editor.value?.getModel()?.getLineCount() ?? 1) ?? 0,
    });
  } else {
    editor.value?.setPosition({ lineNumber: 1, column: 1 });
  }
}
function blur() {
  // no op?
}

defineExpose({ focus, blur, focused: innerFocused, markPosition, width: editorContainerWidth });
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
