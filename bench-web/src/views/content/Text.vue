<script lang="ts" setup>
import { TextData, ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { onMounted, ref, toRef, watch } from "vue";
import { makeViewId } from "@/views";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { keymap } from "prosemirror-keymap";
import { baseKeymap } from "prosemirror-commands";
import { whenever } from "@vueuse/core";
import { PROSE_MIRROR_SCHEMA, type TextMarkType, mapTextToPmNode, mapPmNodeToText } from "@/system/text";
import { type ActionMapImplementation, type ActionImplementation, ACTION_COMING_SOON } from "@/system/action";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: TextData } & Pick<
    ViewData,
    "title" | "icon" | "variant" | "nodePtr"
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
    schema: PROSE_MIRROR_SCHEMA,
    plugins: [keymap(baseKeymap)],
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
      console.log("pm.dispatchTransaction", transaction, updatedText); // nocheckin
      emit("update:modelValue", updatedText);
    },
  });
});

function formatAction(mark: TextMarkType): ActionImplementation {
  return {
    action: ACTION_COMING_SOON,
  };
}
const actions: ActionMapImplementation<"text"> = {
  "text.format.bold": formatAction("bold"),
  "text.format.italic": formatAction("italic"),
  "text.format.strikethrough": formatAction("strikethrough"),
  "text.format.underline": formatAction("underline"),
  "text.format.code": formatAction("code"),
};

canvas.registerView(self, id);

function focus() {
  view!.focus();
}

defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div>
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
		<!-- NOTE: textRef must be in a stable fragment to mount the editor view -->
    <div ref="textRef" class="px-2 py-0.5 focus-within:border-primary-400"></div>
  </div>
</template>
<style>
@import url("/node_modules/prosemirror-view/style/prosemirror.css");

.ProseMirror-focused {
  outline: none;
}
</style>
