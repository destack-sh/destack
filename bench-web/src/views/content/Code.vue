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
import { indentWithTab } from "@codemirror/commands";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { mapCodeToCmDoc, mapPmDocToCode } from "@/system/code";

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
    extensions: [python(), syntaxHighlighting(defaultHighlightStyle), keymap.of([indentWithTab])],
  });
}

function makeEditorView(): EditorView {
  if (codeRef.value == null) throw new Error("codeRef not mounted");
  return new EditorView({
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
    // insert node mention at position (surrounded by spaces)
    // nocheckin: Code.onDrop
  },
});

// actions
const actions: Partial<ActionMapImplementation<"common" | "code">> = {};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div>
    <!-- TODO :UX: Code menus  -->
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
    <!-- NOTE: textRef must be in a stable fragment to mount the editor view -->
    <div
      ref="codeRef"
      class="rounded-md bg-gray-100 py-1 hover:cursor-text"
      :class="[
        variant != Variant.STEALTH ? 'border border-gray-200 focus-within:border-primary-400' : '',
        isInDropZone ? 'outline-dashed outline-2 outline-primary-400' : '',
      ]"
      :draggable="true"
      @dragstart.stop.prevent="false /* prevent accidentally dragging ancestors from text selection here */"
      v-contextmenu="
        (context: MenuContext): OverlayMenuInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(['common.edit.copy', 'common.edit.cut', 'common.edit.paste'], { context }),
          context,
          dontFocus: true, // keep focus on the editor
        })
      "
    />
  </div>
</template>
<style>
/* Code */
.cm-editor .cm-content {
  @apply rounded-md py-0 font-mono;
}
.cm-editor.cm-focused .cm-content {
  @apply outline-0;
}
</style>
