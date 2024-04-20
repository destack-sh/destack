<script lang="ts" setup>
import { ViewData, NodeType, CodeData, Variant } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { onBeforeUnmount, ref, toRef, watch } from "vue";
import { makeViewId } from "@/views";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { useDropZone } from "@/utils/drag";
import type { ActionMapImplementation } from "@/system/action";
import { menuActionsLike, type MenuContext, type OverlayMenuInfo } from "@/utils/menu";
import { deepValueEquals } from "@/utils/ref";
import { whenever } from "@vueuse/core";
import { python } from "@codemirror/lang-python";
import * as commands from "@codemirror/commands";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { mapCodeToCmDoc, mapPmDocToCode } from "@/system/code";
import { autocompletion } from "@codemirror/autocomplete";
import { Casing, toCasing } from "@/utils/string";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: CodeData } & Pick<
    ViewData,
    "title" | "icon" | "variant" | "nodePtr" | "isInput"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const codeRef = ref<HTMLElement | null>(null);
let view: EditorView | null = null;
let lastAppliedModelValue: CodeData | null = null;

function makeEditorState(code?: CodeData): EditorState {
  return EditorState.create({
    doc: code != null ? mapCodeToCmDoc(code) : undefined,
    extensions: [EditorView.lineWrapping, syntaxHighlighting(defaultHighlightStyle), autocompletion({}), python()],
  });
}

function makeEditorView(): EditorView {
  if (codeRef.value == null) throw new Error("codeRef not mounted");
  const view = new EditorView({
    state: makeEditorState(props.modelValue),
    parent: codeRef.value,
    dispatchTransactions(trs, view) {
      // update the state directly for responsiveness
      view.update(trs);
      if (trs.some((tr) => tr.docChanged)) {
        const updatedCode = mapPmDocToCode(view.state.doc, props.modelValue);
        lastAppliedModelValue = updatedCode;
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
  lastAppliedModelValue = props.modelValue ?? null;
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
  const updatedState = makeEditorState(props.modelValue);
  view.setState(updatedState);
});

// drag/drop
const { isInDropZone } = useDropZone({
  name: "text",
  container: codeRef,
  enabled: toRef(props, "isInput"),
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
  "common.move.left": {
    action: () => {
      if (view == null) return;
      commands.indentLess({ state: view.state, dispatch: view.dispatch });
    },
  },
  "common.move.right": {
    action: () => {
      if (view == null) return;
      commands.indentMore({ state: view.state, dispatch: view.dispatch });
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
  <div>
    <!-- TODO :UX: Code menus (autocomplete, refactor, etc.)  -->
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
    <!-- NOTE: textRef must be in a stable fragment to mount the editor view -->
    <div
      ref="codeRef"
      class="code rounded-md bg-gray-100 py-1 hover:cursor-text"
      :class="[
        variant != Variant.STEALTH
          ? 'border border-gray-200 focus-within:border-primary-900'
          : 'outline-1 outline-primary-900 focus-within:outline-dashed',
        isInDropZone ? 'outline-dashed outline-2 outline-primary-400' : '',
      ]"
      :draggable="true"
      @dragstart.stop.prevent="false /* prevent accidentally dragging ancestors from text selection here */"
      v-contextmenu="
        (context: MenuContext): OverlayMenuInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(['code.*', 'common.edit.copy', 'common.edit.cut', 'common.edit.paste'], { context }),
          context,
          dontFocus: true, // keep focus on the editor
        })
      "
    />
  </div>
</template>
<style>
/* Code */
.code .cm-editor .cm-content {
  @apply rounded-md py-0 font-mono;
}
.code .cm-editor.cm-focused {
  @apply outline-0;
}
.code .cm-editor .cm-tooltip {
  @apply overflow-hidden rounded-md border border-gray-400 bg-white p-1 font-mono text-gray-900;
}
.code .cm-editor .cm-tooltip > ul > li {
  @apply rounded-md border border-transparent px-0.5 py-0.5;
}
.code .cm-editor .cm-tooltip > ul > li[aria-selected] {
  @apply bg-primary-300 text-gray-900;
}
.code .cm-editor .cm-completionMatchedText {
  @apply font-semibold underline underline-offset-2;
}
</style>
