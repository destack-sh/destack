<script lang="ts" setup>
import { mapCodeToCmDoc, mapPmDocToCode } from "@/language/code";
import { CodeData, NodeType, Variant, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { useDropZone } from "@/ui/drag";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { copy, cyrb53a } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
import { Casing, toCasing } from "@/utils/string";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { autocompletion } from "@codemirror/autocomplete";
import * as commands from "@codemirror/commands";
import { python } from "@codemirror/lang-python";
import { defaultHighlightStyle, indentUnit, syntaxHighlighting } from "@codemirror/language";
import { EditorSelection, EditorState, StateEffect } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { whenever } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, toRef, watch } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; modelValue?: CodeData } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "variant" | "orientation" | "nodePtr" | "isInput">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const codeRef = ref<HTMLElement | null>(null);
let view: EditorView | null = null;
let lastAppliedModelValue: CodeData | null = null;
const previousSelectionByState: Record<number, EditorSelection> = {};

function makeEditorState(code?: CodeData, options?: { restoreSelection?: boolean }): EditorState {
  let selection: EditorSelection | undefined = undefined;
  if (options?.restoreSelection) {
    selection = previousSelectionByState[cyrb53a(code)];
  }
  const baseExtensions = [
    EditorView.lineWrapping,
    syntaxHighlighting(defaultHighlightStyle),
    autocompletion({}),
    python(),
    indentUnit.of("    "), // 4 spaces
    keymap.of([...commands.defaultKeymap, commands.indentWithTab]),
  ];
  const dynamicExtensions = computed(() => {
    const extensions = [];
    if (props.variant != Variant.STEALTH) {
      extensions.push(lineNumbers());
    }
    if (!props.isInput) {
      extensions.push(EditorView.editable.of(false));
    }
    return extensions;
  });

  const state = EditorState.create({
    doc: code != null ? mapCodeToCmDoc(code) : undefined,
    selection,
    extensions: baseExtensions.concat(dynamicExtensions.value),
  });
  watch(dynamicExtensions, () => {
    if (view?.state !== state) return;
    view.dispatch({
      effects: StateEffect.reconfigure.of(baseExtensions.concat(dynamicExtensions.value)),
    });
  });

  return state;
}

function makeEditorView(): EditorView {
  if (codeRef.value == null) throw new Error("codeRef not mounted");
  const view = new EditorView({
    state: makeEditorState(props.modelValue),
    parent: codeRef.value,
    dispatchTransactions(txs, view) {
      // update the state directly for responsiveness
      view.update(txs);
      const updatedCode = mapPmDocToCode(view.state.doc, props.modelValue);
      if (view?.state.selection != null) {
        // remember selection for this state
        previousSelectionByState[cyrb53a(updatedCode)] = view?.state.selection;
      }
      if (txs.some((tx) => tx.docChanged)) {
        lastAppliedModelValue = updatedCode ?? null;
        emit("update:modelValue", updatedCode);
      }
    },
  });
  // NOTE: apply data-suppress-actions directly to contenteditable element so that our action system finds it
  view.contentDOM.dataset.suppressActions = "common.edit,common.navigate,common.select";
  return view;
}

// mount the editor view
whenever(codeRef, () => {
  if (view) throw new Error("view already exists");
  lastAppliedModelValue = copy(props.modelValue ?? null);
  view = makeEditorView();
});
onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});

// override state from modelValue if different
watch(toRef(props, "modelValue"), () => {
  if (view == null) return;
  if (deepValueEquals(props.modelValue, lastAppliedModelValue)) return;
  const updatedState = makeEditorState(props.modelValue, { restoreSelection: true });
  view.setState(updatedState);
  lastAppliedModelValue = copy(props.modelValue) ?? null;
});

// drag/drop
const { isInDropZone } = useDropZone({
  name: "text",
  container: codeRef,
  isEnabled: toRef(props, "isInput"),
  kinds: ["node"],
  onDrop: (dragged, event) => {
    if (view == null || dragged.kind != "node") return;
    const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (pos == null) return;
    // insert node mention at position (pretty crude right now)
    if (!("name" in dragged.nodes[0])) return;
    const mention = toCasing(dragged.nodes[0].name as string, Casing.SNAKE);
    view.dispatch({
      changes: { from: pos, to: pos, insert: mention },
      selection: { anchor: pos + mention.length },
    });
  },
});

// actions
const actions: Partial<ActionMapImplementation<"common" | "code">> = {
  "common.edit.copy": {
    isEnabled: () => false,
    action: () => {
      throw new Error("not implemented");
    },
  },
  "common.edit.cut": {
    isEnabled: () => false,
    action: () => {
      throw new Error("not implemented");
    },
  },
  "common.edit.paste": {
    isEnabled: () => false,
    action: () => {
      throw new Error("not implemented");
    },
  },
  "common.move.up": {
    action: () => {
      if (view == null) return;
      commands.moveLineUp({ state: view.state, dispatch: view.dispatch });
    },
  },
  "common.move.down": {
    action: () => {
      if (view == null) return;
      commands.moveLineDown({ state: view.state, dispatch: view.dispatch });
    },
  },
  "code.edit.format": {
    isEnabled: () => false,
    action: () => {
      throw new Error("not implemented");
    },
  },
  "code.edit.comment": {
    action: () => {
      if (view == null) return;
      commands.toggleLineComment({ state: view.state, dispatch: view.dispatch });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <ViewContentWrapper :type="ViewType.CODE" :title="title" :variant="variant" :orientation="orientation">
    <!-- NOTE: codeRef must be in a stable fragment to mount the editor view -->
    <div
      ref="codeRef"
      v-contextmenu="
        (context: PopoverContext): PopoverInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(['code.*', 'common.edit.copy', 'common.edit.cut', 'common.edit.paste'], { context }),
          context,
          dontFocus: true, // keep focus on the editor
        })
      "
      data-suppress-actions="common.move.left,common.move.right"
      data-suppress-drag="true"
      class="code rounded hover:cursor-text"
      :class="[
        variant != Variant.STEALTH
          ? 'border border-gray-200 px-1 py-[3px] focus-within:border-primary-700 not-focus-within:hover:border-gray-200'
          : 'stealth',
        isInDropZone ? 'outline-dotted outline-2 outline-gray-400' : '',
      ]"
    />
  </ViewContentWrapper>
</template>
<style>
/* Code */
.code .cm-editor .cm-content {
  @apply rounded py-0 font-mono;
}
.code .cm-editor.cm-focused {
  @apply outline-0;
}
.code .cm-editor .cm-tooltip {
  @apply overflow-hidden rounded border border-gray-200 bg-white p-1 font-mono text-gray-900;
}
.code .cm-editor .cm-tooltip > ul > li {
  @apply rounded border border-transparent px-0.5 py-0.5;
}
.code .cm-editor .cm-tooltip > ul > li[aria-selected] {
  @apply text-primary-700;
}
.code .cm-editor .cm-completionMatchedText {
  @apply font-semibold underline underline-offset-3;
}
.code .cm-editor .cm-gutters {
  @apply mr-0.5 bg-transparent pr-1.5 text-gray-400;
}
.code:focus-within .cm-editor .cm-gutters {
  @apply border-primary-700 text-gray-700;
}
.stealth .ͼ1 .cm-line {
  padding: 0;
}
</style>
