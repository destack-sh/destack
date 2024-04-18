<script lang="ts" setup>
import { NodeType, TextData, Variant, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { type ActionImplementation, type ActionMapImplementation } from "@/system/action";
import { canvas } from "@/system/space";
import { PM_INPUT_RULES, PM_SCHEMA, mapPmNodeToText, mapTextToPmNode, type TextMarkType } from "@/system/text";
import { menuActionsLike, type MenuContext, type OverlayMenuInfo } from "@/utils/menu";
import { deepValueEquals } from "@/utils/ref";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { inputRules } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { getCurrentInstance, onMounted, onUpdated, ref, toRef, watch } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: TextData } & Pick<
    ViewData,
    "title" | "icon" | "variant" | "nodePtr" | "isInput"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const textRef = ref<HTMLDivElement | null>(null);
let view: EditorView | null = null;

function makeEditorState(text?: TextData) {
  return EditorState.create({
    doc: props.modelValue != null ? mapTextToPmNode(props.modelValue, undefined) : undefined,
    schema: PM_SCHEMA,
    plugins: [keymap(commands.baseKeymap), inputRules({ rules: PM_INPUT_RULES })],
  });
}

// overwrite state from modelValue if changed and not focused
watch(
  () => props.modelValue,
  () => {
    if (view == null || view.hasFocus()) return;
    const updatedState = makeEditorState(props.modelValue);
    view.updateState(updatedState);
  },
);

// mount the editor view
whenever(textRef, () => {
  if (view) throw new Error("view already exists");
  view = new EditorView(textRef.value, {
    state: makeEditorState(props.modelValue),
    dispatchTransaction(transaction) {
      // update the state directly for responsiveness & performance
      const newState = view!.state.apply(transaction);
      view!.updateState(newState);
      const updatedText = mapPmNodeToText(newState.doc, props.modelValue);
      // PM triggers transactions even when only the selection changes?
      if (!deepValueEquals(updatedText, props.modelValue)) {
        console.log("pm.dispatchTransaction", transaction, updatedText); // nocheckin
        emit("update:modelValue", updatedText);
      }
    },
  });
});

function formatAction(mark: TextMarkType): ActionImplementation {
  return {
    isEnabled: () => props.isInput,
    isChecked: () => {
      if (!view || !view.state) return false; 
      const { from, to } = view.state.selection;
      let hasMark = false;
      view.state.doc.nodesBetween(from, to, (node) => {
        if (node.marks.some((markType) => markType.type.name === mark)) {
          hasMark = true;
        }
      });
      return hasMark;
    },
    action: () => {
      if (view == null) throw new Error("view not mounted");
      const { state, dispatch } = view;
      if (state.selection.empty) {
        // switch to that mark
        dispatch(state.tr.setStoredMarks([state.schema.marks[mark].create()]));
      } else {
        // add or remove the mark from the selection
        commands.toggleMark(state.schema.marks[mark])(state, dispatch);
      }
    },
  };
}
const actions: ActionMapImplementation<"text"> & Partial<ActionMapImplementation<"common">> = {
  // text
  "text.format.bold": formatAction("bold"),
  "text.format.italic": formatAction("italic"),
  "text.format.strikethrough": formatAction("strikethrough"),
  "text.format.underline": formatAction("underline"),
  "text.format.code": formatAction("code"),
  // common
  "common.select.all": {
    action: () => commands.selectAll(view!.state, view!.dispatch),
  },
};

function focus() {
  view!.focus();
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH], actions, focus });
</script>
<template>
  <div>
    <!-- TODO :UX: Text menus (insert, morph, bubble, etc.) -->
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
    <!-- NOTE: textRef must be in a stable fragment to mount the editor view -->
    <div
      ref="textRef"
      class="prose rounded-md"
      :class="[variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-0.5 focus-within:border-primary-400' : '']"
      v-contextmenu="
        (context: MenuContext): OverlayMenuInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(
            ['text.*', 'common.edit.morph', 'common.edit.copy', 'common.edit.cut', 'common.edit.paste'],
            { context },
          ),
          context,
          dontFocus: true, // keep focus on the editor
        })
      "
    />
  </div>
</template>
<style>
/* Prose */
.prose {
  @apply text-gray-900;
}
.prose hr {
  @apply my-2 border-gray-700 focus:outline-none focus:ring-0;
}
.prose h1 {
  @apply mb-1.5 mt-3 text-2xl font-semibold;
}
.prose h2 {
  @apply mb-1 mt-2 text-xl font-semibold;
}
.prose h3 {
  @apply mb-0.5 mt-1 text-lg font-semibold;
}
</style>
<style>
/* PM */
@import url("/node_modules/prosemirror-view/style/prosemirror.css");

.ProseMirror-focused {
  outline: none;
}
</style>
