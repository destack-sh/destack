<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import { downloadFile, prefetchFile, uploadFile } from "@/language/file";
import { isTextEmpty, mapPmNodeToText, mapTextToPmNode } from "@/language/text";
import {
  BenchType,
  ColorShade,
  NodeReferenceData,
  NodeType,
  ObjectType,
  TextData,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData
} from "@/proto/wire";
import {
  isNodeRef,
  toNodeRef,
  type TypedNodeReferenceData
} from "@/proto/wiring";
import { bench, canvas, pkgConnection, pkgGraph } from "@/system/space";
import { IS_IN_ALT_MODE, type ActionImplementation, type ActionMapImplementation } from "@/ui/action";
import { useDropZone } from "@/ui/drag";
import { DEFAULT_MISSING_ICON, ICON_BY_NODE_TYPE, getNodeIcon } from "@/ui/icon";
import {
  menuActionsLike,
  popPopover,
  pushPopover,
  trackHoverElementOnce,
  type PopoverContext,
  type PopoverInfo,
  type PopoverInstance,
} from "@/ui/popover";
import { getColorHex } from "@/ui/style";
import { toaster } from "@/ui/toast";
import { getElement } from "@/utils/element";
import { copy, cyrb53a } from "@/utils/functools";
import { log } from "@/utils/log";
import { PM_INPUT_RULES, PM_KEYMAP_EXTRA, PM_SCHEMA, type TextMarkType } from "@/utils/prosemirror";
import { deepValueEquals } from "@/utils/ref";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { dropCursor } from "prosemirror-dropcursor";
import { inputRules } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { Node as PmNode } from "prosemirror-model";
import {
  Selection as EditorSelection,
  EditorState,
  type SelectionBookmark as EditorSelectionBookmark,
} from "prosemirror-state";
import { EditorView, type NodeView as PmNodeView } from "prosemirror-view";
import { computed, nextTick, onBeforeUnmount, ref, toRef, watch, type Ref } from "vue";

const MENTION_TRIGGER_CHAR = "@";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: TextData;
    placeholder?: string;
    suppressEnter?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "variant" | "nodePtr" | "isInput">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const textRef = ref<HTMLDivElement | null>(null);
let view: EditorView | null = null;
let lastAppliedModelValue: TextData | null = null;
const previousSelectionByState: Record<number, EditorSelectionBookmark> = {};

const mentionPtrs: Ref<NodeReferenceData[]> = computed(() => {
  const mentionPtrs: NodeReferenceData[] = [];
  for (const line of props.modelValue?.lines ?? []) {
    for (const span of line.spans ?? []) {
      if (span.nodePtr != null) mentionPtrs.push(span.nodePtr);
    }
  }
  return mentionPtrs;
});
// TODO :Incomplete: some mentioned nodes may not be in package graph for Text
//  (use supergraph? but when to load missing nodes?)
const mentions = pkgGraph.getManyRef(mentionPtrs);
function resolveMention(mention: { id: string; ck: string; nodeType: NodeType }): AnyNodeData | null {
  const node = pkgGraph.get(mention);
  if (node != null) return node;
  // find rich reference
  const ref = mentionPtrs.value.find((r) => r.id == mention.id || r.ck == mention.ck) ?? null;
  return ref as AnyNodeData | null;
}

function makeEditorState(text?: TextData, options?: { restoreSelection?: boolean }): EditorState {
  const doc = text != null ? mapTextToPmNode(text, undefined) : undefined;
  let selection: EditorSelection | undefined = undefined;
  if (options?.restoreSelection && doc != null) {
    selection = previousSelectionByState[cyrb53a(text)]?.resolve(doc);
  }
  return EditorState.create({
    doc: doc,
    schema: PM_SCHEMA,
    selection,
    plugins: [
      keymap({ ...commands.baseKeymap, ...PM_KEYMAP_EXTRA, ...(props.suppressEnter ? { Enter: () => true } : {}) }),
      inputRules({ rules: PM_INPUT_RULES }),
    ],
  });
}

function makeEditorView(): EditorView {
  return new EditorView(textRef.value, {
    state: makeEditorState(props.modelValue),
    // TODO :UX: non-editable Text should be selectable
    editable: () => props.isInput,
    nodeViews: {
      mention: (node, view, getPos) => new MentionView(node, view),
    },
    plugins: [dropCursor({ width: 2, color: "#fbbf24" })],
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

      // trigger mention if we just typed the trigger char
      const { selection } = newState;
      if (tx.docChanged && selection.empty && selection.$head.nodeBefore?.text?.endsWith(MENTION_TRIGGER_CHAR)) {
        const referencePos = view.coordsAtPos(selection.$head.pos);
        pushPopover({
          trigger: getElement(textRef.value)!,
          reference: { x: referencePos.left, y: referencePos.top },
          info: {
            component: ViewType.PICKER,
            referenceMargin: 2,
            offset: { x: 0, y: -8 }, // align query text with line
            placement: "inside-top-left",
            props: { placeholder: "Mention Node", valueType: makeTypeInfo({ benchType: BenchType.BLOCK }) },
            onApply(node) {
              if (view == null) throw new Error("view no longer mounted");
              // replace @ with mention and focus there
              const mention = PM_SCHEMA.node("mention", { nodePtr: toNodeRef(node) });
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

  constructor(pmNode: PmNode, view: EditorView) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("mention");
    this.dom.dataset.nodeType = pmNode.attrs.nodePtr.nodeType;
    this.dom.dataset.nodeId = pmNode.attrs.nodePtr.id;
    this.dom.dataset.nodeCk = pmNode.attrs.nodePtr.ck;
    this.iconDom = this.dom.appendChild(document.createElement("span"));
    this.iconDom.classList.add(
      "icon",
      ...(ICON_BY_NODE_TYPE[pmNode.attrs.nodePtr.nodeType as NodeType]?.faName?.split(" ") ?? ["fas", "fa-question"]),
    );
    this.nameDom = this.dom.appendChild(document.createElement("span"));
    this.nameDom.classList.add("name");
    this.nameDom.textContent = "???";

    // click
    this.dom.addEventListener("click", async () => {
      // go to mention on alt-click
      if (IS_IN_ALT_MODE.value) {
        canvas.goToNode(pmNode.attrs.nodePtr);
      } else if (pmNode.attrs.nodePtr.nodeType == NodeType.FILE) {
        // open file on click
        const download = downloadFile(pmNode.attrs.nodePtr);
        try {
          await download.completion.wait();
          if (download.getUrl.value == null) throw new Error(`missing GET url`);
        } catch (e) {
          log.error("text.file.download.error", download, e);
          toaster.error({
            title: "Download Failed",
            text: `'${pmNode.attrs.nodePtr.title}': ${(e as any).message ?? "unknown error"}`,
          });
        }
        window.open(download.getUrl.value!, "_blank");
      }
    });

    // open file preview on hover
    // (would be nice to have this be more general :NodePreviews)
    if (pmNode.attrs.nodePtr.nodeType == NodeType.FILE) {
      let popoverInstance: PopoverInstance | undefined;
      this.dom.addEventListener("mouseenter", () => {
        // prefetch (to speed up load on hover)
        prefetchFile(pmNode.attrs.nodePtr);
        // keep preview open on hover
        trackHoverElementOnce(this.dom, {
          getOtherElements: () => (popoverInstance?.element != null ? [popoverInstance.element] : []),
          onHover: () => {
            if (popoverInstance != null) {
              return;
            }
            popoverInstance = pushPopover({
              trigger: this.dom,
              reference: this.dom,
              info: {
                placement: "bottom",
                component: ViewType.FILE,
                props: {
                  modelValue: pmNode.attrs.nodePtr,
                  isInline: true,
                  size: { metatype: ObjectType.RECTANGLE, width: 400 },
                },
              },
            });
          },
          onLeave: () => {
            if (popoverInstance != null) {
              popPopover(popoverInstance);
              popoverInstance = undefined;
            }
          },
          immediate: true,
        });
      });
    }

    const node = resolveMention(pmNode.attrs.nodePtr);
    if (node != null) this.updateNode(node);
  }

  updateNode(node: AnyNodeData) {
    // content
    const nodeType = isNodeRef(node) ? node.nodeType : node.metatype;
    this.nameDom.textContent = (node as any).name ?? (node as any).title ?? "???";

    // style
    const icon = getNodeIcon(node) ?? DEFAULT_MISSING_ICON;
    this.iconDom.className = icon?.faName != null ? `icon ${icon.faName}` : "icon fa fa-question";
    if (icon.color != null) this.iconDom.style.color = getColorHex(icon.color, ColorShade.S600)!;
    else this.iconDom.style.removeProperty("color");
    this.dom.dataset.nodeType = NodeType[nodeType].toLowerCase();
  }
}

// sync mentions with mention views
watch(
  mentions,
  () => {
    if (view == null) return;
    view.dom.querySelectorAll(".mention").forEach((mentionDom) => {
      if (!(mentionDom instanceof HTMLElement)) return;
      const node = resolveMention({
        nodeType: Number.parseInt(mentionDom.dataset.nodeType!),
        id: mentionDom.dataset.nodeId!,
        ck: mentionDom.dataset.nodeCk!,
      });
      const pmView = (mentionDom as any).__pmView as MentionView;
      if (node == null || pmView == null) return;
      pmView.updateNode(node);
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
  isEnabled: toRef(props, "isInput"),
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
defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <ViewContentWrapper :type="ViewType.TEXT" v-bind="props">
    <!-- NOTE :UX :Incomplete: Text menus (insert, morph, bubble, etc.) -->
    <!-- NOTE: textRef must be in a stable fragment to mount the editor view -->
    <div
      ref="textRef"
      v-contextmenu="
        (context: PopoverContext): PopoverInfo => ({
          isEnabled: props.isInput,
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(['text.*', 'common.edit.copy', 'common.edit.cut', 'common.edit.paste'], { context }),
          context,
          dontFocus: true, // keep focus on the editor
        })
      "
      class="text relative rounded hover:cursor-text"
      :class="[
        variant != Variant.STEALTH
          ? 'border border-gray-200 px-2 py-0.5 focus-within:border-gray-400 not-focus-within:hover:border-gray-200'
          : 'stealth',
        isInDropZone ? 'outline-dotted outline-2 outline-gray-400' : '',
      ]"
      data-suppress-actions="common.move.left,common.move.right"
      data-suppress-drag="true"
    >
      <!-- Placeholder -->
      <div v-if="placeholder && isTextEmpty(modelValue)" class="pointer-events-none absolute left-2 top-1">
        <div class="text-sm text-gray-400">{{ placeholder }}</div>
      </div>
    </div>
  </ViewContentWrapper>
</template>
<style>
/* Prose */
.text {
  @apply text-gray-900;
  line-height: 1.65;
}
.text strong {
  @apply font-semibold;
}
.text .line:first-child {
  @apply mt-0; /* ignore top margin */
}
.text .line:last-child {
  @apply mb-0; /* ignore bottom margin */
}
.text p {
  @apply my-[4px];
}
.text hr {
  @apply my-2 border-gray-200 p-0 focus:outline-none focus:ring-0;
}
.text h1 {
  @apply mb-2 mt-4 text-2xl font-bold;
  line-height: 1.2;
}
.text h2 {
  @apply mb-1.5 mt-2.5 text-xl font-bold;
  line-height: 1.4;
}
.text h3 {
  @apply mb-0.5 mt-1.5 text-lg font-bold;
  line-height: 1.5;
}
.text h4 {
  @apply mb-0.5 mt-1.5 text-base font-medium;
  line-height: 1.5;
}
.text code {
  @apply rounded bg-gray-100 px-0.5;
}
.text blockquote {
  @apply my-2 border-l-4 border-gray-400 py-[1px] pl-2;
}
.text div.callout {
  @apply my-2 rounded bg-gray-100 px-2 py-2.5;
}
/* TODO: UI: callout/heading/etc. line icons should be editable */
.text div.callout::before {
  content: "\f06a"; /* fa-icon: circle-exclamation */
  font-family: "Font Awesome 6 Pro";
  font-weight: 900;
  @apply mr-1 px-1 text-gray-700;
}

/* Mentions */
.text span.mention {
  @apply rounded px-1 py-0;
}
.text span.mention:hover {
  @apply bg-gray-100;
}
.text span.mention .icon {
  @apply mr-1.5 text-gray-700;
}
.text span.mention:hover .icon {
  @apply text-gray-700;
}
.text span.mention .name {
  @apply underline decoration-gray-300 underline-offset-3;
}
.text span.textMirror-selectednode.mention .name {
  @apply bg-primary-100 text-primary-700 decoration-primary-700;
}
.altmode .text span.mention:hover,
.text span.mention[data-node-type="file"]:hover .name {
  @apply cursor-pointer;
}
.altmode .text span.mention:hover .name,
.text span.mention[data-node-type="file"]:hover .name {
  @apply decoration-primary-700;
}
</style>
<style>
/* PM */
@import url("/node_modules/prosemirror-view/style/prosemirror.css");

.ProseMirror-focused {
  outline: none;
}
.ProseMirror-selectednode {
  @apply p-2 outline-primary-700;
}
</style>
