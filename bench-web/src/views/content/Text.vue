<script lang="ts" setup>
import { supergraph } from "@/globals";
import { getBaseFromNodeReference } from "@/language/core/const";
import { isTextEmpty } from "@/language/core/text";
import { uploadFile } from "@/language/resource/file";
import { ColorShade, NodeReferenceData, NodeType, ObjectType, TextData, ViewData } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { bench, canvas, pkgConnection } from "@/system/space";
import { type ActionImplementation, type ActionMapImplementation } from "@/ui/action";
import { useDropZone } from "@/ui/drag";
import { DEFAULT_MISSING_ICON, getNodeIcon, getNodeName, ICON_BY_NODE_TYPE } from "@/ui/icon";
import {
  mapPmNodeToText,
  mapTextToPmNode,
  PM_INPUT_RULES,
  PM_KEYMAP_EXTRA,
  PM_SCHEMA,
  type TextMarkType,
} from "@/ui/prosemirror";
import { getColorHex } from "@/ui/style";
import { copy, cyrb53a } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { dropCursor } from "prosemirror-dropcursor";
import { inputRules } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { Node as PmNode } from "prosemirror-model";
import {
  Command,
  Selection as EditorSelection,
  EditorState,
  Plugin,
  TextSelection,
  type SelectionBookmark as EditorSelectionBookmark,
} from "prosemirror-state";
import { EditorView, type NodeView as PmNodeView } from "prosemirror-view";
import { computed, onBeforeUnmount, ref, toRef, watch, type Ref } from "vue";

const NODE_TRIGGER_CHAR = "@";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: TextData;
    placeholder?: string;
    suppressEnter?: boolean;
    suppressDrop?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isMinimal">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const textRef = ref<HTMLDivElement | null>(null);
let view: EditorView | null = null;
let lastAppliedModelValue: TextData | null = null;
const previousSelectionByState: Record<number, EditorSelectionBookmark> = {};

const nodePtrs: Ref<NodeReferenceData[]> = computed(() => {
  const nodePtrs: NodeReferenceData[] = [];
  for (const line of props.modelValue?.lines ?? []) {
    for (const span of line.spans ?? []) {
      if (span.nodePtr != null) nodePtrs.push(span.nodePtr);
    }
  }
  return nodePtrs;
});
const basePtrs = computed(() => nodePtrs.value.map((ptr) => getBaseFromNodeReference(ptr)).filter((b) => b != null));
const bases = supergraph.getManyRef(basePtrs);
const mentions = supergraph.getManyRef(nodePtrs);

function makeEditorState(text?: TextData, options?: { restoreSelection?: boolean }): EditorState {
  const doc = text != null ? mapTextToPmNode(text, undefined) : undefined;
  let selection: EditorSelection | undefined = undefined;
  if (options?.restoreSelection && doc != null) {
    selection = previousSelectionByState[cyrb53a(text)]?.resolve(doc);
  }
  const bindings: Record<string, Command> = {
    ...commands.baseKeymap,
    ...PM_KEYMAP_EXTRA,
  };
  if (props.suppressEnter) {
    bindings["Shift-Enter"] = commands.baseKeymap["Enter"];
    bindings.Enter = () => true;
  }
  return EditorState.create({
    doc: doc,
    schema: PM_SCHEMA,
    selection,
    plugins: [keymap(bindings), inputRules({ rules: PM_INPUT_RULES })],
  });
}

function makeEditorView(): EditorView {
  const plugins: Plugin[] = [];
  if (!props.suppressDrop) {
    plugins.push(dropCursor({ width: 2, color: "#fbbf24" }));
  }
  return new EditorView(textRef.value, {
    state: makeEditorState(props.modelValue),
    editable: () => props.isInput,
    nodeViews: {
      mention: (node, view, getPos) => new MentionView(node, view),
    },
    plugins,
    dispatchTransaction(tx) {
      if (view == null) throw new Error("view not mounted");

      // update the state directly for responsiveness & performance
      const newState = view.state.apply(tx);
      view.updateState(newState);
      const updatedText = mapPmNodeToText(newState.doc, props.modelValue);
      if (view?.state.selection != null) {
        // remember selection for this state
        previousSelectionByState[cyrb53a(updatedText)] = newState.selection.getBookmark();
      }
      // also update the modelValue if underlying doc changed
      if (tx.docChanged) {
        lastAppliedModelValue = updatedText;
        emit("update:modelValue", updatedText);
      }
    },
  });
}

/** Mini-component for PM mentions */
class MentionView implements PmNodeView {
  dom: HTMLElement;
  nodePtr: NodeReferenceData;
  iconDom: HTMLElement;
  nameDom: HTMLElement;

  constructor(pmNode: PmNode, view: EditorView) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("mention");
    this.dom.dataset.nodeType = pmNode.attrs.nodePtr.nodeType;
    this.dom.dataset.nodeId = pmNode.attrs.nodePtr.id;
    this.dom.dataset.nodeCk = pmNode.attrs.nodePtr.ck;
    this.nodePtr = {
      ...pmNode.attrs.nodePtr,
      nodeType: Number(pmNode.attrs.nodePtr.nodeType),
      metatype: ObjectType.NODE_REFERENCE,
    };
    this.iconDom = this.dom.appendChild(document.createElement("span"));
    this.iconDom.classList.add(
      "icon",
      ...(ICON_BY_NODE_TYPE[pmNode.attrs.nodePtr.nodeType as NodeType]?.faName?.split(" ") ?? ["fas", "fa-question"]),
    );
    this.nameDom = this.dom.appendChild(document.createElement("span"));
    this.nameDom.classList.add("name");
    this.nameDom.textContent = "???";

    this.updateMention();
  }

  updateMention() {
    const nodePtr = this.nodePtr;
    const node = supergraph.get(nodePtr);
    this.nameDom.textContent = (node != null ? getNodeName(node) : null) ?? "???";
    const icon = (node != null ? getNodeIcon(node) : null) ?? DEFAULT_MISSING_ICON;
    this.iconDom.className = icon?.faName != null ? `icon ${icon.faName}` : "icon fa fa-question";
    if (icon.color != null) this.iconDom.style.color = getColorHex(icon.color, ColorShade.S600)!;
    else this.iconDom.style.removeProperty("color");
    this.dom.dataset.nodeType = nodePtr.nodeType.toString();
  }
}

// sync mentions with mention views
watch(
  mentions,
  () => {
    if (view == null) return;
    view.dom.querySelectorAll(".mention").forEach((mentionDom) => {
      if (!(mentionDom instanceof HTMLElement)) return;
      const pmView = (mentionDom as any).__pmView as MentionView;
      if (pmView == null) return;
      pmView.updateMention();
    });
  },
  { immediate: true },
);

// mount the editor view
whenever(textRef, () => {
  if (view) throw new Error("view already exists");
  lastAppliedModelValue = copy(props.modelValue ?? null);
  view = makeEditorView();
});
onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});

// overwrite state from modelValue if different
watch(toRef(props, "modelValue"), () => {
  if (view == null) return;
  if (deepValueEquals(props.modelValue, lastAppliedModelValue)) return;
  const updatedState = makeEditorState(props.modelValue, { restoreSelection: true });
  view.updateState(updatedState);
  lastAppliedModelValue = props.modelValue ?? null;
});

function insertMention(nodePtr: NodeReferenceData, pos: { pos: number }) {
  if (view == null) throw new Error("view not mounted");
  const pmNode = PM_SCHEMA.node("mention", { nodePtr });
  view.dispatch(view.state.tr.insert(pos.pos, pmNode).insertText(" ", pos.pos + 1, pos.pos + 1));
}

// drag/drop
const { isInDropZone } = useDropZone({
  name: "text",
  container: textRef,
  isEnabled: computed(() => props.isInput && !props.suppressDrop),
  kinds: ["node", "file"],
  onDrop: (dragged, event) => {
    if (view == null) return;
    if (dragged.kind == "file") {
      // upload files and insert as mentions at position (surrounded by spaces)
      if (dragged.files == null) return;
      const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
      if (pos == null) return; // not in editor
      Array.from(dragged.files).forEach(async (file) => {
        // upload and insert each file individually
        if (bench.value == null) throw new Error("no current bench");
        const upload = uploadFile(() => pkgConnection.tx, file, { bench: bench.value });
        await upload.completion.wait();
        if (view == null) throw new Error("view no mounted");
        insertMention(toNodeRef(upload.file.value!), pos);
      });
    } else if (dragged.kind == "node") {
      // insert node mention at position (surrounded by spaces)
      const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
      if (pos == null) return; // not in editor
      insertMention(toNodeRef(dragged.node), pos);
    }
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
const actions: ActionMapImplementation<"text"> & Partial<ActionMapImplementation<"space">> = {
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
  // space
  "space.edit.delete": {
    action: () => commands.deleteSelection(view!.state, view!.dispatch),
  },
  "space.select.all": {
    action: () => commands.selectAll(view!.state, view!.dispatch),
  },
};

function focus() {
  if (view) {
    const { state } = view;
    const end = state.doc.content.size;
    view.focus();
    view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, end)));
  }
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text relative rounded hover:cursor-text"
    :class="[
      !isMinimal
        ? 'border border-gray-200 px-2 py-0.5 focus-within:border-gray-400 not-focus-within:hover:border-gray-200'
        : 'stealth',
      isInDropZone ? 'outline-dotted outline-2 outline-gray-400' : '',
    ]"
    data-suppress-actions="space.move.left,space.move.right"
    data-suppress-drag="both"
  >
    <!-- Placeholder -->
    <div
      v-if="placeholder && isTextEmpty(modelValue)"
      class="pointer-events-none absolute"
      :class="!isMinimal ? 'left-2 top-1' : 'left-0.5 top-0.5'"
    >
      <div class="text-sm text-gray-400">{{ placeholder }}</div>
    </div>
  </div>
</template>

