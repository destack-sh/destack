import { uploadFile } from "@/language/resource/file";
import { NodeReferenceData, TextLineType, TextSpanType } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { bench, pkgConnection } from "@/system/space";
import { type ActionImplementation, type ActionMapImplementation } from "@/ui/action";
import { useDropZone } from "@/ui/drag";
import { PM_SCHEMA, TextMarkType } from "@/ui/prosemirror/schema";
import { BlockRenderer, SpanNodeView } from "@/ui/prosemirror/view";
import { LineInterface, mapPmNodeToText, mapTextToPmNode, TextInterface } from "@/ui/prosemirror/wiring";
import { deepValueEquals } from "@/utils/ref";
import { FocusAnchor, NavigationDirection } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { dropCursor } from "prosemirror-dropcursor";
import { ellipsis, emDash, InputRule, inputRules, smartQuotes } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import {
  NodeRange,
  NodeType as PmNodeType,
  Node as PmNode,
  ResolvedPos,
  type MarkType as PmMarkType,
} from "prosemirror-model";
import {
  Command,
  EditorState,
  NodeSelection,
  Plugin,
  PluginKey,
  Transaction as PmTransaction,
  TextSelection,
  type SelectionBookmark as EditorSelectionBookmark,
} from "prosemirror-state";
import { liftTarget } from "prosemirror-transform";
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import {
  Component,
  ComponentInternalInstance,
  computed,
  MaybeRef,
  onBeforeUnmount,
  shallowRef,
  toValue,
  triggerRef,
  watch,
  type Ref,
} from "vue";

//
// Editor
//

/** Highlighting plugin */
const highlightPluginKey = new PluginKey("highlightPlugin");
export function useHighlightPlugin(options: { selectedBlockIds: Ref<string[]> }) {
  const { selectedBlockIds } = options;

  return new Plugin({
    key: highlightPluginKey,
    state: {
      init(_config, { doc }) {
        return DecorationSet.empty;
      },
      apply(tr, oldDecos, oldState, newState) {
        // recompute decorations based on the external highlightedBlockIds
        const decorations: Decoration[] = [];
        newState.doc.descendants((node, pos) => {
          if (node.attrs.blockPtr != null && selectedBlockIds.value.includes(node.attrs.blockPtr.id)) {
            decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: "selected" }));
          }
        });
        return DecorationSet.create(newState.doc, decorations);
      },
    },
    props: {
      decorations(state) {
        return this.getState(state);
      },
    },
  });
}

/** Wrap a range in a list container, merging with adjacent lists of the same type if possible. */
function wrapInList(tr: PmTransaction, range: NodeRange, type: TextLineType) {
  const listContainerType =
    type === TextLineType.LIST_ORDERED ? PM_SCHEMA.nodes.orderedList : PM_SCHEMA.nodes.unorderedList;
  const posAfter = tr.doc.resolve(range.start);
  if (posAfter.depth < 1 || posAfter.node(posAfter.depth - 1).type !== listContainerType) {
    const range = posAfter.blockRange();
    if (range) {
      tr.wrap(range, [{ type: listContainerType }]);
    }
  }
}

/**
 * Change the type of a line node, lifting as needed.
 * If the current node is inside a list item, it will first split the list container
 * (if necessary) to isolate the current list item, then lift that item out into the grandparent,
 * and finally change its type to the target type.
 * The selection is preserved by retaining the original horizontal offset.
 */
function morphLineNode(
  state: EditorState,
  $from: ResolvedPos,
  dispatch: (tr: PmTransaction) => void,
  targetType: TextLineType,
): boolean {
  const schema = state.schema;
  const tr = state.tr;

  // map target node type
  let targetNodeType: PmNodeType;
  switch (targetType) {
    case TextLineType.PARAGRAPH:
      targetNodeType = schema.nodes.lineParagraph;
      break;
    case TextLineType.HEADING_1:
    case TextLineType.HEADING_2:
    case TextLineType.HEADING_3:
    case TextLineType.HEADING_4:
      targetNodeType = schema.nodes.lineHeading;
      break;
    case TextLineType.LIST_ORDERED:
      targetNodeType = schema.nodes.lineListOrdered;
      break;
    case TextLineType.LIST_UNORDERED:
      targetNodeType = schema.nodes.lineListUnordered;
      break;
    default:
      return false;
  }

  // if already the target type, do nothing
  if ($from.parent.type === targetNodeType && $from.parent.attrs.type === targetType) {
    return false;
  }

  // if current block is not a list item, simply set its type
  if ($from.parent.type.name !== "lineListOrdered" && $from.parent.type.name !== "lineListUnordered") {
    tr.setBlockType($from.start(), $from.end(), targetNodeType, {
      ...$from.parent.attrs,
      type: targetType,
    });
    // preserve horizontal offset
    const offset = $from.parentOffset;
    const $newPos = tr.doc.resolve($from.pos);
    const newStart = $newPos.start($newPos.depth);
    const newOffset = Math.min(offset, $newPos.parent.content.size);
    tr.setSelection(TextSelection.create(tr.doc, newStart + newOffset));
    dispatch?.(tr);
    return true;
  }

  // we're inside a list item
  // get the list container (one level up) and its boundaries
  const listItem = $from.parent;
  const listContainer = $from.node($from.depth - 1);
  const containerStart = $from.before($from.depth - 1);
  const containerEnd = $from.after($from.depth - 1);
  const index = $from.index($from.depth - 1);

  // if list container has only one item, replace it with the new node
  if (listContainer && listContainer.childCount === 1) {
    const newNode = targetNodeType.create({ ...listItem.attrs, type: targetType }, listItem.content);
    tr.replaceWith(containerStart, containerEnd, newNode);
    tr.setSelection(TextSelection.near(tr.doc.resolve(containerStart + 1)));
    dispatch?.(tr);
    return true;
  }

  // if list container has multiple items, isolate the current list item by splitting before and after if necessary
  let $itemStart = tr.doc.resolve($from.before($from.depth));
  let $itemEnd = tr.doc.resolve($from.after($from.depth));
  if (listContainer.childCount > 1) {
    if (index > 0) {
      // split before the current list item
      tr.split($from.before($from.depth));
    }
    // remap current position after split
    const mappedPos1 = tr.mapping.map($from.pos);
    let $mapped = tr.doc.resolve(mappedPos1);
    const newContainer = $mapped.node($mapped.depth - 1);
    if (newContainer.childCount > 1) {
      // split after the current list item
      tr.split($mapped.after($mapped.depth));
    }
    const mappedPos = tr.mapping.map($from.pos);
    $mapped = tr.doc.resolve(mappedPos);
    $itemStart = tr.doc.resolve($mapped.before($mapped.depth));
    $itemEnd = tr.doc.resolve($mapped.after($mapped.depth));
  }

  // lift the isolated list item out of its parent into the grandparent
  const range = new NodeRange(tr.doc.resolve($itemStart.pos), tr.doc.resolve($itemEnd.pos), $itemStart.depth);
  const liftTargetValue = liftTarget(range);
  if (liftTargetValue == null) return false;
  tr.lift(range, liftTargetValue);

  // map the original position after lifting
  const posAfterLift = tr.mapping.map($from.pos);
  const $newPos = tr.doc.resolve(posAfterLift);
  // change the type of the now lifted node to the target type
  tr.setBlockType($newPos.start($newPos.depth), $newPos.end($newPos.depth), targetNodeType, {
    ...listItem.attrs,
    type: targetType,
  });

  // preserve the horizontal offset from the original selection
  const offset = $from.parentOffset;
  const newStart = $newPos.start($newPos.depth);
  const newOffset = Math.min(offset, $newPos.parent.content.size);
  tr.setSelection(TextSelection.create(tr.doc, newStart + newOffset));
  dispatch?.(tr);
  return true;
}

/** Define a prefix pattern that creates a Line. */
function linePrefixRule(pattern: string | RegExp, nodeType: PmNodeType, type: TextLineType) {
  const regexp = typeof pattern === "string" ? new RegExp(`^(${pattern})$`) : pattern;
  const rule = new InputRule(regexp, (state, match, start, end) => {
    const { tr } = state;
    const $start = tr.doc.resolve(start);
    const block = $start.parent; // keep existing attrs, set new type
    tr.setBlockType(start, end, nodeType, { ...block.attrs, type });
    tr.delete(start, end);
    tr.setSelection(TextSelection.near(tr.doc.resolve(start)));

    // wrap list items in an orderedList/unorderedList if not already in one.
    if (type === TextLineType.LIST_ORDERED || type === TextLineType.LIST_UNORDERED) {
      wrapInList(tr, new NodeRange($start, $start, $start.depth), type);
    }

    return tr;
  });
  return rule;
}

const lineDividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start - 1, end, state.schema.nodes.lineDivider.create());
  tr.insert(start, state.schema.nodes.lineParagraph.create());
  tr.setSelection(TextSelection.near(tr.doc.resolve(end)));
  return tr;
});

/** Replace inline text with something. */
function replacementRule(pattern: RegExp, replacement: string) {
  const rule = new InputRule(pattern, replacement);
  return rule;
}

/**
 * Create an input rule that transforms text wrapped in markers into text with a given mark (applying the mark only within the region).
 * For example, markerRule('*', PM_SCHEMA.marks.em) will convert "*text*" into italicized text.
 */
function markerRule(marker: string, markType: PmMarkType) {
  const escapedMarker = marker.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const char = marker[0].replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  let regex: RegExp;
  if (marker.length == 1) {
    regex = new RegExp(`(?:^|[^*])${escapedMarker}(?!${char})(.+?)${escapedMarker}(?!${char})$`);
  } else if (marker.length == 2) {
    regex = new RegExp(`(?:^|[^*])${escapedMarker}(?!${char})(.+?)${escapedMarker}(?!${char})$`);
  } else {
    throw new Error(`unsupported marker: ${marker}`);
  }
  const rule = new InputRule(regex, (state, match, start, end) => {
    const { tr, schema } = state;
    const text = match[1];
    tr.replaceWith(start, end, schema.text(text, [markType.create()]));
    tr.setSelection(TextSelection.create(tr.doc, start + text.length));
    tr.removeStoredMark(markType);
    return tr;
  });
  return rule;
}

const UNORDERED_LIST_CHARS = ["-", "\\*", "•"];
const PM_INPUT_RULES: InputRule[] = [
  // character rules
  emDash,
  ellipsis,
  ...smartQuotes,
  replacementRule(/\(c\)/, "©"),
  replacementRule(/->/, "→"),
  replacementRule(/>>/, "»"),
  replacementRule(/!=/, "≠"),
  replacementRule(/\(tm\)/, "™"),
  replacementRule(/\(r\)/, "®"),
  replacementRule(/<3/, "❤️"),
  replacementRule(/<=>/, "⇔"),
  replacementRule(/<=/, "≤"),
  replacementRule(/>=/, "≥"),
  // marker rules
  markerRule("*", PM_SCHEMA.marks.italic),
  markerRule("_", PM_SCHEMA.marks.italic),
  markerRule("**", PM_SCHEMA.marks.bold),
  markerRule("__", PM_SCHEMA.marks.bold),
  markerRule("~", PM_SCHEMA.marks.strikethrough),
  markerRule("~~", PM_SCHEMA.marks.strikethrough),
  // line rules
  linePrefixRule("# ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_1),
  linePrefixRule("## ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_2),
  linePrefixRule("### ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_3),
  linePrefixRule("#### ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_4),
  lineDividerRule,
  ...UNORDERED_LIST_CHARS.map((char) =>
    linePrefixRule(`${char} `, PM_SCHEMA.nodes.lineListUnordered, TextLineType.LIST_UNORDERED),
  ),
  ...UNORDERED_LIST_CHARS.map((char) =>
    linePrefixRule(` ${char} `, PM_SCHEMA.nodes.lineListUnordered, TextLineType.LIST_UNORDERED),
  ),
  linePrefixRule(/^[0-9a-z]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule(/^\s[0-9a-z]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule("> ", PM_SCHEMA.nodes.lineQuote, TextLineType.QUOTE),
  linePrefixRule("! ", PM_SCHEMA.nodes.lineCallout, TextLineType.CALLOUT),
  linePrefixRule("``` ", PM_SCHEMA.nodes.lineCode, TextLineType.CODE),
];

/** Build the ProseMirror commands. */
function getPmCommands(options: { navigate: (direction: NavigationDirection) => void; deleteSelf: () => void }) {
  const { navigate, deleteSelf } = options;
  const extraCommands: Record<string, Command> = {
    ArrowLeft(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[0] &&
        (state.doc.textContent === "" || view?.endOfTextblock("left", state))
      ) {
        navigate("left");
        return true;
      }
      return false;
    },
    ArrowRight(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[state.doc.children.length - 1] &&
        (state.doc.textContent === "" || view?.endOfTextblock("right", state))
      ) {
        navigate("right");
        return true;
      }
      return false;
    },
    ArrowUp(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[0] &&
        (state.doc.textContent === "" || view?.endOfTextblock("up", state))
      ) {
        navigate("up");
        return true;
      }
      return false;
    },
    ArrowDown(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[state.doc.children.length - 1] &&
        (state.doc.textContent === "" || view?.endOfTextblock("down", state))
      ) {
        navigate("down");
        return true;
      }
      return false;
    },
    Backspace(state, dispatch) {
      const { empty, $cursor, $from, $to } = state.selection as TextSelection;
      if (
        $from.parent.type.name !== "lineParagraph" &&
        $from.parentOffset === 0 &&
        $from.start() === $from.end() &&
        $to.pos === $from.end()
      ) {
        // morph to plain paragraph
        return morphLineNode(state, $from, dispatch!, TextLineType.PARAGRAPH);
      } else if (empty && state.doc.textContent === "" && state.doc.children.length <= 1) {
        // delete self
        deleteSelf();
        return true;
      } else {
        // imitate default behavior
        return commands.chainCommands(
          commands.deleteSelection,
          commands.joinBackward,
          commands.selectNodeBackward,
        )(state, dispatch);
      }
    },
    "Mod-Enter": (state, dispatch, view) => commands.splitBlock(state, dispatch),
    Enter(state, dispatch, view) {
      const { $from, $to } = state.selection;
      const parentType = $from.parent.type.name;
      if (parentType === "lineListOrdered" || parentType === "lineListUnordered") {
        if ($from.parent.textContent.trim() === "") {
          // morph to plain paragraph
          return morphLineNode(state, $from, dispatch!, TextLineType.PARAGRAPH);
        } else {
          // otherwise, split the list item at the cursor position (keep attrs except blockPtr)
          const tr = state.tr.split($from.pos, 1, [
            { type: $from.parent.type, attrs: { ...$from.parent.attrs, blockPtr: null } },
          ]);
          dispatch?.(tr);
          return true;
        }
      }
      return commands.splitBlock(state, dispatch);
    },
  };
  return extraCommands;
}

/**
 * Install a Text editor on a DOM element.
 */
export function useTextEditor(options: {
  textRef: Ref<HTMLElement | null>;
  text: TextInterface;
  isInput: MaybeRef<boolean>;
  suppressEnter: MaybeRef<boolean>;
  suppressDrop: MaybeRef<boolean>;
  navigate: (direction: NavigationDirection) => void;
  deleteSelf: () => void;
  plugins?: Plugin[];
  parentComponent: ComponentInternalInstance;
  blockComponent: Component;
}) {
  // nocheckin: maintain selection across state changes (for undo/redo)
  const previousSelectionByState: Record<number, EditorSelectionBookmark> = {};
  const { textRef, text, isInput, suppressEnter, suppressDrop, navigate, deleteSelf } = options;

  // state
  const lines = computed(() => text.read());
  function makeEditorState(stateIn: { lines: LineInterface[]; selection?: EditorSelectionBookmark }): EditorState {
    const doc = mapTextToPmNode(stateIn.lines);
    const bindings: Record<string, Command> = {
      ...commands.baseKeymap,
      ...getPmCommands({ navigate, deleteSelf }),
    };
    if (toValue(suppressEnter)) {
      bindings["Shift-Enter"] = bindings.Enter;
      bindings["Mod-Enter"] = () => true;
      bindings.Enter = () => true;
    }
    const selection = stateIn.selection?.resolve(doc);
    const plugins: Plugin[] = [keymap(bindings), inputRules({ rules: PM_INPUT_RULES })];
    if (options?.plugins != null) {
      plugins.push(...options.plugins);
    }
    const state = EditorState.create({
      doc: doc,
      schema: PM_SCHEMA,
      selection,
      plugins,
    });
    return state;
  }

  // view utils
  const lineRefsById: Ref<Record<string, HTMLElement>> = shallowRef({});
  function updateLineRefs(view: EditorView) {
    lineRefsById.value = {};
    view.dom.querySelectorAll("[data-node-id]").forEach((dom) => {
      const id = (dom as HTMLElement).dataset?.nodeId;
      if (id != null) {
        lineRefsById.value[id] = dom as HTMLElement;
      }
    });
    triggerRef(lineRefsById);
  }
  function findLineNodeById(id: string): { node: PmNode | null; pos: number | null } {
    let targetNode: PmNode | null = null;
    let targetPos: number | null = null;
    view?.state.doc.descendants((node, pos) => {
      if (node.attrs.blockPtr?.id == id) {
        targetNode = node;
        targetPos = pos;
      }
    });
    return { node: targetNode, pos: targetPos };
  }

  // view
  let view: EditorView | null = null;
  let lastAppliedModelValue: LineInterface[] = [];
  function makeEditorView(): EditorView {
    const plugins: Plugin[] = [];
    if (!options.suppressDrop) {
      plugins.push(dropCursor({ width: 2, color: "#fbbf24" }));
    }
    const view = new EditorView(textRef.value, {
      state: makeEditorState({ lines: lines.value }),
      editable: () => toValue(isInput),
      nodeViews: {
        spanNode: (node, view, getPos) => new SpanNodeView(node, view),
        block: (node, view, getPos) =>
          new BlockRenderer({
            component: options.blockComponent,
            node,
            view,
            getPos,
            navigate,
            parentComponent: options.parentComponent,
          }),
      },
      plugins,
      dispatchTransaction(tx) {
        if (view == null) throw new Error("view not mounted");
        // update the state
        let newState = view.state.apply(tx);
        const updatedText = mapPmNodeToText(newState.doc);
        // also write the doc (if changed)
        if (tx.docChanged) {
          lastAppliedModelValue = text.write(updatedText);
          if (lastAppliedModelValue !== updatedText) {
            // re-derive state in case we modified the doc (like when creating a new line)
            // NOTE :Performance: re-deriving state on every write seems wasteful (chugging fine so far though)
            newState = makeEditorState({ lines: lastAppliedModelValue, selection: newState.selection.getBookmark() });
          }
        }
        view.updateState(newState);
        if (tx.docChanged) {
          updateLineRefs(view);
        }
      },
    });
    updateLineRefs(view);
    return view;
  }

  // mount the editor view
  whenever(textRef, () => {
    if (view) throw new Error("view already exists");
    lastAppliedModelValue = lines.value.map((l) => l);
    view = makeEditorView();
  });
  onBeforeUnmount(() => {
    view?.destroy();
    view = null;
  });

  // overwrite state from modelValue if different
  watch(lines, () => {
    if (view == null) return;
    if (deepValueEquals(lines.value, lastAppliedModelValue)) return; // already applied
    const updatedState = makeEditorState({ lines: lines.value });
    view.updateState(updatedState);
    lastAppliedModelValue = lines.value;
  });

  /** Insert a Node mention at a position. */
  function insertNode(nodePtr: NodeReferenceData, pos: { pos: number }) {
    if (view == null) throw new Error("view not mounted");
    const pmNode = PM_SCHEMA.node("spanNode", { type: TextSpanType.NODE, nodePtr });
    view.dispatch(view.state.tr.insert(pos.pos, pmNode).insertText(" ", pos.pos + 1, pos.pos + 1));
  }

  // drag/drop
  const { isInDropZone } = useDropZone({
    name: "text",
    container: textRef,
    isEnabled: computed(() => toValue(isInput) && !toValue(suppressDrop)),
    kinds: ["node", "file"],
    onDrop: (dragged, event) => {
      if (view == null) return;
      if (dragged.kind === "file") {
        // upload files and insert as nodes at position (surrounded by spaces)
        if (dragged.files == null) return;
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        Array.from(dragged.files).forEach(async (file) => {
          // upload and insert each file individually
          if (bench.value == null) throw new Error("no current bench");
          const upload = uploadFile(() => pkgConnection.tx, file, { bench: bench.value });
          await upload.completion.wait();
          if (view == null) throw new Error("view not mounted");
          insertNode(toNodeRef(upload.file.value!), pos);
        });
      } else if (dragged.kind === "node") {
        // insert node at position (surrounded by spaces)
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        insertNode(toNodeRef(dragged.node), pos);
      }
    },
  });

  // formatting
  function markFormatAction(mark: TextMarkType): ActionImplementation {
    return {
      isEnabled: () => toValue(isInput),
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
  function spanTypeFormatAction(type: TextSpanType): ActionImplementation {
    return {
      isEnabled: () => toValue(isInput),
      isChecked: () => false,
      action: () => {
        if (view == null) throw new Error("view not mounted");
        // nocheckin: span type format action
      },
    };
  }

  // actions
  const actions: ActionMapImplementation<"text"> & Partial<ActionMapImplementation<"space">> = {
    // text
    "text.format.bold": markFormatAction("bold"),
    "text.format.italic": markFormatAction("italic"),
    "text.format.strikethrough": markFormatAction("strikethrough"),
    "text.format.underline": markFormatAction("underline"),
    "text.format.code": spanTypeFormatAction(TextSpanType.CODE),
    "text.format.equation": spanTypeFormatAction(TextSpanType.EQUATION),
    "text.edit.hardBreak": {
      action: () => {
        // insert 'hardBreak' node at cursor
        if (view == null || toValue(suppressEnter)) return false;
        const { from } = view.state.selection;
        const hardBreak = PM_SCHEMA.node("spanHardBreak");
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

  // focus
  function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
    if (view == null) return; // nothing to do
    const { state } = view;
    view.focus();
    let selection;
    if (typeof anchor == "object") {
      // focus node with id
      const { node: targetNode, pos: targetPos } = findLineNodeById(anchor.id!);
      if (targetNode == null) {
        selection = TextSelection.atEnd(state.doc);
      } else if (targetNode.type.name == "block") {
        selection = NodeSelection.create(state.doc, targetPos!);
      } else {
        const $endPos = state.doc.resolve(targetPos!);
        selection = new TextSelection($endPos);
      }
    } else if (anchor === "top" || anchor === "left") {
      selection = TextSelection.atStart(state.doc);
    } else {
      selection = TextSelection.atEnd(state.doc);
    }
    view.dispatch(state.tr.setSelection(selection));
  }

  // (force) update plugin state
  function updatePlugin(plugin: Plugin) {
    if (view == null) return; // nothing to do
    const tr = view.state.tr;
    tr.setMeta("plugin", { forceUpdate: Date.now() });
    view.dispatch(tr);
  }
  return { focus, actions, isInDropZone, updatePlugin, lineRefsById };
}
