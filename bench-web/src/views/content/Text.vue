<script lang="ts" setup>
import { NodeType, TextData, Variant, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { type ActionImplementation, type ActionMapImplementation } from "@/system/action";
import { canvas } from "@/system/space";
import { mapPmNodeToText, mapTextToPmNode } from "@/system/text";
import { useDropZone, useSingleDropZone } from "@/utils/drag";
import { menuActionsLike, type MenuContext, type OverlayMenuInfo } from "@/utils/menu";
import { PM_SCHEMA, PM_INPUT_RULES, type TextMarkType } from "@/utils/prosemirror";
import { deepValueEquals } from "@/utils/ref";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { inputRules } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { Plugin, EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { onBeforeUnmount, ref, toRef, watch } from "vue";

const MENTION_TRIGGER_CHAR = "@";

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
let lastAppliedModelValue: TextData | null = null;

function makeEditorState(text?: TextData) {
  return EditorState.create({
    doc: text != null ? mapTextToPmNode(text, undefined) : undefined,
    schema: PM_SCHEMA,
    plugins: [
      keymap(commands.baseKeymap),
      inputRules({ rules: PM_INPUT_RULES }),
      new Plugin({
        // suggestion/mention plugin
      }),
    ],
  });
}

// overwrite state from modelValue if changed and not focused
watch(
  () => props.modelValue,
  () => {
    if (view == null) return;
    if (deepValueEquals(props.modelValue, lastAppliedModelValue)) return;
    const updatedState = makeEditorState(props.modelValue);
    view.updateState(updatedState);
  },
);

// mount the editor view
whenever(textRef, () => {
  if (view) throw new Error("view already exists");
  lastAppliedModelValue = props.modelValue ?? null;
  view = new EditorView(textRef.value, {
    state: makeEditorState(props.modelValue),
    dispatchTransaction(transaction) {
      // update the state directly for responsiveness & performance
      const newState = view!.state.apply(transaction);
      view!.updateState(newState);
      if (transaction.docChanged) {
        const updatedText = mapPmNodeToText(newState.doc, props.modelValue);
        lastAppliedModelValue = updatedText;
        emit("update:modelValue", updatedText);
      }
    },
  });
});
onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});

// drag/drop
const { isInDropZone } = useDropZone({
  name: "text",
  container: textRef,
  enabled: toRef(props, "isInput"),
  kinds: ["node"],
  onDrop: (dragged) => {
    console.log("text drop", dragged); // nocheckin
  },
});

function formatAction(mark: TextMarkType): ActionImplementation {
  return {
    isEnabled: () => props.isInput,
    isChecked: () => {
      if (view == null) return false;
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
      commands.toggleMark(view.state.schema.marks[mark])(view.state, view.dispatch);
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
  "common.edit.delete": {
    action: () => commands.deleteSelection(view!.state, view!.dispatch),
  },
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
      :class="[
        variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-0.5 focus-within:border-primary-400' : '',
        isInDropZone ? 'rounded-md bg-primary-200' : '',
      ]"
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
.prose p:not(:first-of-type):not(:last-of-type) {
  @apply my-0.5;
}
.prose hr {
  @apply my-2 border-gray-700 p-0 focus:outline-none focus:ring-0;
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
.prose code {
  @apply rounded-md bg-gray-100 px-0.5;
}
.prose blockquote {
  @apply my-2 border-l-2 border-gray-700 py-[1px] pl-2;
}
.prose div.callout {
  @apply my-2 rounded-md  bg-gray-100 px-2 py-2.5;
}
/* TODO: UI: callout/heading/etc. line icons should be editable */
.prose div.callout::before {
  content: "\f06a"; /* fa-icon: exclamation-circle */
  font-family: "Font Awesome 6 Pro";
  @apply mr-1 px-1 text-gray-700;
}
</style>
<style>
/* PM */
@import url("/node_modules/prosemirror-view/style/prosemirror.css");

.ProseMirror-focused {
  outline: none;
}
.ProseMirror-selectednode {
  @apply p-2 outline-primary-400;
}
</style>
