<script lang="ts" setup>
import { BenchType, NodeReferenceData, NodeType, TextData, Variant, ViewData, type AnyNodeData } from "@/proto/wire";
import { toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { IS_IN_ALT_MODE, type ActionImplementation, type ActionMapImplementation } from "@/system/action";
import { ICON_BY_NODE_TYPE, getNodeIcon } from "@/system/icon";
import { canvas, pkgGraph } from "@/system/space";
import { mapPmNodeToText, mapTextToPmNode } from "@/system/text";
import { useDropZone } from "@/utils/drag";
import { createOverlayMenu, menuActionsLike, type MenuContext, type OverlayMenuInfo } from "@/utils/menu";
import { PM_INPUT_RULES, PM_SCHEMA, type TextMarkType } from "@/utils/prosemirror";
import { deepValueEquals } from "@/utils/ref";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { inputRules } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { Node as PmNode } from "prosemirror-model";
import { EditorState } from "prosemirror-state";
import { EditorView, type NodeView as PmNodeView } from "prosemirror-view";
import { dropCursor } from "prosemirror-dropcursor";
import { computed, nextTick, onBeforeUnmount, ref, toRef, watch } from "vue";
import { getElement } from "@/utils/element";
import Picker from "@/views/content/Picker.vue";
import { makeTypeInfo } from "@/system/value";

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
const mentionPtrs = computed(() => {
  const mentionPtrs: NodeReferenceData[] = [];
  for (const line of props.modelValue?.lines ?? []) {
    for (const span of line.spans ?? []) {
      if (span.nodePtr != null) mentionPtrs.push(span.nodePtr);
    }
  }
  return mentionPtrs;
});
// TODO :Incomplete: some mentioned nodes may not be in package graph for Text
const mentions = pkgGraph.getManyRef(mentionPtrs);
function resolveMention(mention: { id: string; ck?: string }): AnyNodeData | null {
  return pkgGraph.get(mention);
}

function makeEditorState(text?: TextData) {
  return EditorState.create({
    doc: text != null ? mapTextToPmNode(text, undefined) : undefined,
    schema: PM_SCHEMA,
    plugins: [keymap(commands.baseKeymap), inputRules({ rules: PM_INPUT_RULES })],
  });
}

function makeEditorView(): EditorView {
  return new EditorView(textRef.value, {
    state: makeEditorState(props.modelValue),
    nodeViews: {
      mention: (node, view, getPos) => new MentionView(node, view, getPos),
    },
    plugins: [dropCursor({ width: 2, color: "#fbbf24" })],
    dispatchTransaction(transaction) {
      if (view == null) throw new Error("view not mounted");

      // update the state directly for responsiveness & performance
      const newState = view.state.apply(transaction);
      view.updateState(newState);
      // also update the modelValue if underlying doc changed
      if (transaction.docChanged) {
        const updatedText = mapPmNodeToText(newState.doc, props.modelValue);
        lastAppliedModelValue = updatedText;
        emit("update:modelValue", updatedText);
      }

      // trigger mention if we just typed the trigger char
      const { selection } = newState;
      if (
        transaction.docChanged &&
        selection.empty &&
        selection.$head.nodeBefore?.text?.endsWith(MENTION_TRIGGER_CHAR)
      ) {
        const referencePos = view.coordsAtPos(selection.$head.pos);
        createOverlayMenu({
          trigger: getElement(textRef.value)!,
          reference: { x: referencePos.left, y: referencePos.top },
          info: {
            kind: "component",
            component: Picker,
            referenceMargin: 2,
            referenceOffset: { x: 0, y: -10 }, // align query text with line
            placement: "inside-top-left",
            props: {
              valueType: makeTypeInfo({ benchType: BenchType.BLOCK }),
              isInline: true,
            },
            onApply(node) {
              if (view == null) throw new Error("view no longer mounted");
              // replace @ with mention and focus there
              const mention = PM_SCHEMA.node("mention", { nodePtr: toNodeReference(node) });
              view.dispatch(
                view.state.tr
                  .delete(selection.$head.pos - 1, selection.$head.pos)
                  .insert(selection.$head.pos - 1, mention)
                  .insert(selection.$head.pos, PM_SCHEMA.text(" ")),
              );
            },
            onClose: () => {
              if (view == null) throw new Error("view no longer mounted");
              nextTick(() => view!.focus());
            },
          },
        });
      }
    },
  });
}

/** Mini-component for PM mentions */
class MentionView implements PmNodeView {
  dom: HTMLElement;
  iconDom: HTMLElement;
  nameDom: HTMLElement;

  constructor(pmNode: PmNode, view: EditorView, getPos: () => number | undefined) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("mention");
    this.dom.dataset.nodeType = pmNode.attrs.nodePtr.type;
    this.dom.dataset.nodeId = pmNode.attrs.nodePtr.id;
    this.dom.dataset.nodeCk = pmNode.attrs.nodePtr.ck;
    this.iconDom = this.dom.appendChild(document.createElement("span"));
    this.iconDom.classList.add(
      "icon",
      ...(ICON_BY_NODE_TYPE[pmNode.attrs.nodePtr.type as NodeType]?.faName?.split(" ") ?? ["fas", "fa-question"]),
    );
    this.nameDom = this.dom.appendChild(document.createElement("span"));
    this.nameDom.classList.add("name");
    this.nameDom.textContent = "???";

    this.dom.addEventListener("click", () => {
      if (IS_IN_ALT_MODE.value) {
        canvas.goToNode(pmNode.attrs.nodePtr);
      }
    });

    const node = resolveMention(pmNode.attrs.nodePtr);
    if (node != null) this.updateNode(node);
  }

  updateNode(node: AnyNodeData) {
    const icon = getNodeIcon(node);
    this.iconDom.className = "";
    this.iconDom.classList.add("icon", ...(icon?.faName?.split(" ") ?? ["fas", "fa-question"]));
    this.nameDom.textContent = (node as any).name ?? "???";
  }
}

// sync mentions with mention views
watch(mentions, () => {
  if (view == null) return;
  view.dom.querySelectorAll(".mention").forEach((mentionDom) => {
    if (!(mentionDom instanceof HTMLElement)) return;
    const node = resolveMention({ id: mentionDom.dataset.id!, ck: mentionDom.dataset.ck });
    if (node == null) return;
    const pmView = (mentionDom as any).__pmView as MentionView;
    pmView.updateNode(node);
  });
});

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
  view = makeEditorView();
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
  onDrop: (dragged, event) => {
    if (view == null || dragged.kind != "node") return;
    // insert node mention at position (surrounded by spaces)
    const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
    if (pos == null) return; // not in editor
    const pmNode = PM_SCHEMA.node("mention", { nodePtr: toNodeReference(dragged.node) });
    view.dispatch(view.state.tr.insert(pos.pos, pmNode).insertText(" ", pos.pos + 1, pos.pos + 1));
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
  "text.edit.hardBreak": {
    action: () => {
      // insert 'hardBreak' node at cursor
      if (view == null) return;
      const { from } = view.state.selection;
      const hardBreak = PM_SCHEMA.node("hardBreak");
      view.dispatch(view.state.tr.insert(from, hardBreak));
    },
  },
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
      class="prose rounded-md hover:cursor-text"
      :class="[
        variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-0.5 focus-within:border-primary-400' : '',
        isInDropZone ? 'outline-dashed outline-2 outline-primary-400' : '',
      ]"
      :draggable="true"
      @dragstart.stop.prevent="false /* prevent accidentally dragging ancestors from text selection here */"
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
  line-height: 1.65;
}
.prose strong {
  @apply font-semibold;
}
.prose p:first-of-type {
  @apply mt-0;
}
.prose p {
  @apply my-[4px];
}
.prose hr {
  @apply my-2 border-gray-700 p-0 focus:outline-none focus:ring-0;
}
.prose h1 {
  @apply mb-2 mt-4 text-2xl font-bold;
  line-height: 1.2;
}
.prose h2 {
  @apply mb-1.5 mt-2.5 text-xl font-bold;
  line-height: 1.4;
}
.prose h3 {
  @apply mb-0.5 mt-1.5 text-lg font-bold;
  line-height: 1.5;
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
  font-weight: 900;
  @apply mr-1 px-1 text-gray-700;
}

/* Mentions */
.prose span.mention {
  @apply rounded-md px-1 py-0;
}
.prose span.mention:hover {
  @apply bg-primary-100 text-primary-900;
}
.prose span.mention .icon {
  @apply mr-1.5 text-gray-700;
}
.prose span.mention:hover .icon {
  @apply text-primary-900;
}
.prose span.mention .name {
  @apply underline-offset-3  underline decoration-gray-300;
}
.prose span.ProseMirror-selectednode.mention .name {
  @apply bg-primary-100 text-primary-900  decoration-primary-900;
}
.altmode .prose span.mention:hover {
  @apply cursor-pointer;
}
.altmode .prose span.mention:hover .name {
  @apply decoration-primary-900;
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
