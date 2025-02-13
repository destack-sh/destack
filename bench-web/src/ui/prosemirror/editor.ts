import { NodeReferenceData, TextLineType, TextSpanType } from "@/proto/wire";
import { type ActionImplementation, type ActionMapImplementation } from "@/ui/action";
import { PageContext } from "@/ui/prosemirror/page";
import { PM_SCHEMA, SpanSpecialInputType, TextMarkType } from "@/ui/prosemirror/schema";
import { LineBlockView, SpanNodeView, VueComponentView } from "@/ui/prosemirror/view";
import { LineInterface, mapPmNodeToText, mapTextToPmNode, TextInterface } from "@/ui/prosemirror/wiring";
import { deepValueEquals } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import TextSpecialInput from "@/views/builtins/TextSpecialInput.vue";
import { FocusAnchor, NavigationDirection } from "@/views/common";
import Block from "@/views/nodes/Block.vue";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { dropCursor } from "prosemirror-dropcursor";
import { history, redo, undo } from "prosemirror-history";
import { ellipsis, emDash, InputRule, inputRules, smartQuotes } from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import {
  NodeRange,
  Node as PmNode,
  NodeType as PmNodeType,
  ResolvedPos,
  type MarkType as PmMarkType,
} from "prosemirror-model";
import {
  Command,
  EditorState,
  NodeSelection,
  Plugin,
  Transaction as PmTransaction,
  Selection,
  TextSelection,
} from "prosemirror-state";
import { liftTarget } from "prosemirror-transform";
import { EditorView } from "prosemirror-view";
import {
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

/** Wrap a range in a list container, merging with adjacent lists of the same type if possible. */
export function wrapInList(tr: PmTransaction, range: NodeRange, type: TextLineType) {
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

/** Wrap a range in a container node if needed. */
export function wrapIfNeeded(tr: PmTransaction, $start: ResolvedPos, type: TextLineType) {
  if (type === TextLineType.LIST_ORDERED || type === TextLineType.LIST_UNORDERED) {
    wrapInList(tr, new NodeRange($start, $start, $start.depth), type);
  }
}

/**
 * Find the first ancestor of the given type.
 */
export function findAncestor(
  state: EditorState,
  $pos: ResolvedPos,
  predicate: (node: PmNode) => boolean,
): { node: PmNode | null; pos: number | null } {
  for (let i = $pos.depth; i >= 0; i--) {
    if (predicate($pos.node(i))) return { node: $pos.node(i), pos: $pos.pos };
  }
  return { node: null, pos: null };
}

/**
 * Change the type of a line node, lifting as needed.
 * If the current node is inside a list item, it will first split the list container
 * (if necessary) to isolate the current list item, then lift that item out into the grandparent,
 * and finally change its type to the target type.
 * The selection is preserved by retaining the original horizontal offset.
 */
export function morphLineNode(
  state: EditorState,
  $from: ResolvedPos,
  targetType: TextLineType,
  dispatch?: (tr: PmTransaction) => void,
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
    wrapIfNeeded(tr, $from, targetType);
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
    wrapIfNeeded(tr, $start, type);
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

/** Replace inline text with a temporary input. */
function specialInputRule(pattern: RegExp, type: SpanSpecialInputType, inlineChar: string | undefined) {
  return new InputRule(pattern, (state, match, start, end) => {
    if (inlineChar != null) {
      // only trigger if the cursor is exactly at the end of the inline char
      if (state.selection.from !== end || match.input == null || match.index == null) return null;
      if (match.input.length > match.index + 1) {
        return null;
      }
      start -= match.input.length - match.index - 1;
    }

    const { tr } = state;
    tr.replaceWith(start, end, PM_SCHEMA.nodes.spanSpecialInput.create({ type }));
    tr.setSelection(NodeSelection.create(tr.doc, start));
    return tr;
  });
}

/** Get a regex for a wrapping marker. */
function getMarkerRegex(marker: string): RegExp {
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
  return regex;
}

/**
 * Create an input rule that transforms text wrapped in markers into text with a given mark.
 */
function markerRule(marker: string, markType: PmMarkType) {
  const regex = getMarkerRegex(marker);
  const rule = new InputRule(regex, (state, match, start, end) => {
    const { tr, schema } = state;
    const text = match[1];
    tr.replaceWith(start, end, schema.text(text, [markType.create()]));
    tr.setSelection(TextSelection.create(tr.doc, start + text.length));
    tr.removeStoredMark(markType); // continue typing without mark
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
  // special input rules
  specialInputRule(/^\/$/, "/", undefined),
  specialInputRule(/@/, "@", "@"),
  // marker rules
  markerRule("*", PM_SCHEMA.marks.italic),
  markerRule("_", PM_SCHEMA.marks.italic),
  markerRule("**", PM_SCHEMA.marks.bold),
  markerRule("__", PM_SCHEMA.marks.bold),
  markerRule("~", PM_SCHEMA.marks.strikethrough),
  markerRule("~~", PM_SCHEMA.marks.strikethrough),
  markerRule("`", PM_SCHEMA.marks.code),
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
        (selection.$anchor.parent === state.doc || selection.$anchor.parent === state.doc.children[0]) &&
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
        (selection.$anchor.parent === state.doc ||
          selection.$anchor.parent === state.doc.children[state.doc.children.length - 1]) &&
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
        (selection.$anchor.parent === state.doc || selection.$anchor.parent === state.doc.children[0]) &&
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
        (selection.$anchor.parent === state.doc ||
          selection.$anchor.parent === state.doc.children[state.doc.children.length - 1]) &&
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
        return morphLineNode(state, $from, TextLineType.PARAGRAPH, dispatch);
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
    "Mod-Enter": (state, dispatch, view) => {
      return commands.splitBlockAs((node, atEnd, $from) => {
        if (node.type.isInGroup("line")) {
          // remove blockPtr from the new block (force create)
          return { type: node.type, attrs: { ...node.attrs, blockPtr: null } };
        }
        return null;
      })(state, dispatch);
    },
    Enter(state, dispatch, view) {
      const { $from, $to } = state.selection;
      const parentType = $from.parent.type.name;
      if (parentType === "lineListOrdered" || parentType === "lineListUnordered") {
        if ($from.parent.textContent.trim() === "") {
          // morph to plain paragraph
          return morphLineNode(state, $from, TextLineType.PARAGRAPH, dispatch);
        } else {
          // otherwise, split the list item at the cursor position (keep attrs except blockPtr)
          const tr = state.tr.split($from.pos, 1, [
            { type: $from.parent.type, attrs: { ...$from.parent.attrs, blockPtr: null } },
          ]);
          dispatch?.(tr);
          return true;
        }
      }
      return commands.splitBlockAs((node, atEnd, $from) => {
        if (node.type.isInGroup("line")) {
          // remove blockPtr from the new block (force create)
          return { type: node.type, attrs: { ...node.attrs, blockPtr: null } };
        }
        return null;
      })(state, dispatch);
    },
  };
  return extraCommands;
}

/** Whether any of the selected nodes have the given mark. */
export function hasTextMark(state: EditorState, selection: Selection, mark: TextMarkType): boolean | "mixed" {
  const { from, to } = selection;
  let hasMark = false;
  state.doc.nodesBetween(from, to, (node) => {
    if (node.marks.some((markType) => markType.type.name === mark)) {
      hasMark = true;
    }
  });
  return hasMark;
}

/** Set the mark on the selected nodes. */
export function setTextMark(
  state: EditorState,
  selection: Selection,
  mark: TextMarkType,
  isSet: boolean | "toggle",
  dispatch: (tr: PmTransaction) => void,
) {
  if (isSet === "toggle") {
    commands.toggleMark(state.schema.marks[mark])(state, dispatch);
  } else if (isSet) {
    const tr = state.tr.addMark(selection.from, selection.to, state.schema.marks[mark].create());
    dispatch(tr);
  } else {
    const tr = state.tr.removeMark(selection.from, selection.to, state.schema.marks[mark]);
    dispatch(tr);
  }
}

/** Whether any of the selected nodes have the given span type. */
export function hasTextSpanType(state: EditorState, selection: Selection, type: TextSpanType): boolean | "mixed" {
  const { from, to } = selection;
  let hasSpanType = false;
  state.doc.nodesBetween(from, to, (node) => {
    if (node.type.name.startsWith("span") && node.attrs.type === type) {
      hasSpanType = true;
    }
  });
  return hasSpanType;
}

/** Set the (inner) span type of the selected nodes. */
export function setTextSpanType(
  state: EditorState,
  selection: Selection,
  type: TextSpanType,
  dispatch: (tr: PmTransaction) => void,
) {
  throw new Error("TODO :Incomplete: setTextSpanType not yet supported");
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
  plugins: Plugin[];
  parentComponent: ComponentInternalInstance;
  history?: boolean;
  pageContext?: PageContext;
  onTransaction?: (view: EditorView, prevState: EditorState, newState: EditorState) => void;
}) {
  const { textRef, text, isInput, suppressEnter, suppressDrop, navigate, deleteSelf } = options;

  // setup
  const bindings: Record<string, Command> = {
    ...commands.baseKeymap,
    ...getPmCommands({ navigate, deleteSelf }),
  };
  if (toValue(suppressEnter)) {
    bindings["Shift-Enter"] = bindings.Enter;
    bindings["Mod-Enter"] = () => true;
    bindings.Enter = () => true;
  }
  if (options?.history) {
    bindings["Mod-z"] = undo;
    bindings["Mod-y"] = redo;
    bindings["Mod-Shift-z"] = redo;
  }
  const plugins: Plugin[] = [keymap(bindings), inputRules({ rules: PM_INPUT_RULES })];
  if (options?.plugins != null) {
    plugins.push(...options.plugins);
  }
  if (options?.history) {
    plugins.push(history({ depth: 1000, newGroupDelay: 3000 }));
  }

  // state
  const lines = computed(() => text.read());
  function makeEditorState(lines: LineInterface[]): EditorState {
    const doc = mapTextToPmNode(lines);
    const state = EditorState.create({ doc: doc, schema: PM_SCHEMA, plugins });
    return state;
  }

  // view nodes
  const lineRefsById: Ref<Record<string, HTMLElement>> = shallowRef({});
  function updateLineRefs(view: EditorView) {
    lineRefsById.value = {};
    view.dom.querySelectorAll("[data-node-id].line, [data-node-id].line-block").forEach((lineDom) => {
      const id = (lineDom as HTMLElement).dataset?.nodeId;
      if (id != null) {
        lineRefsById.value[id] = lineDom as HTMLElement;
      }
    });
    triggerRef(lineRefsById);
  }
  function findLineNodeById(state: EditorState, id: string): { node: PmNode | null; pos: number | null } {
    let targetNode: PmNode | null = null;
    let targetPos: number | null = null;
    state.doc.descendants((node, pos) => {
      if (node.attrs.blockPtr?.id == id) {
        targetNode = node;
        targetPos = pos;
      }
    });
    return { node: targetNode, pos: targetPos };
  }

  // view
  let view: EditorView | null = null;
  let prevText: LineInterface[] = [];
  function makeEditorView(): EditorView {
    const plugins: Plugin[] = [];
    if (!options.suppressDrop) {
      plugins.push(dropCursor({ width: 2, color: "#fbbf24" }));
    }
    const view = new EditorView(textRef.value, {
      state: makeEditorState(lines.value),
      editable: () => toValue(isInput),
      nodeViews: {
        spanNode: (node, view, getPos) =>
          new SpanNodeView({
            component: NodeReference,
            parentComponent: options.parentComponent,
            node,
            view,
            getPos,
          }),
        spanSpecialInput: (node, view, getPos) =>
          new VueComponentView({
            style: "inline",
            component: TextSpecialInput,
            parentComponent: options.parentComponent,
            props: {
              type: node.attrs.type,
              node,
              view,
              getPos,
              pageContext: options.pageContext,
            },
            node,
            view,
            getPos,
          }),
        block: (node, view, getPos) =>
          new LineBlockView({
            component: Block,
            parentComponent: options.parentComponent,
            node,
            view,
            getPos,
            navigate,
          }),
      },
      plugins,
      dispatchTransaction(tx) {
        if (view == null) throw new Error("view not mounted");
        // update the state
        const prevState = view.state;
        let newState = view.state.apply(tx);
        const { lines: updatedText, linesNodes, linesPos } = mapPmNodeToText(newState.doc);
        // also write the doc (if changed)
        if (tx.docChanged && !tx.getMeta("_ignoreDocChanged")) {
          prevText = text.write(updatedText);
          // update the blockPtr for modified lines
          if (prevText !== updatedText) {
            let tr = newState.tr;
            for (let i = 0; i < linesNodes.length; i++) {
              const lineNode = linesNodes[i];
              const line = prevText[i];
              if (line.blockPtr?.id !== lineNode.attrs.blockPtr?.id) {
                tr = tr.setNodeAttribute(linesPos[i], "blockPtr", line.blockPtr);
              }
            }
            newState = newState.apply(tr);
          }
        }
        view.updateState(newState);
        if (tx.docChanged) {
          updateLineRefs(view);
        }
        options.onTransaction?.(view, prevState, newState);
      },
    });
    updateLineRefs(view);
    return view;
  }

  // mount the editor view
  whenever(textRef, () => {
    if (view) throw new Error("view already exists");
    prevText = lines.value.slice();
    view = makeEditorView();
  });
  onBeforeUnmount(() => {
    view?.destroy();
    view = null;
  });

  // overwrite state from modelValue if different
  watch(lines, () => {
    if (view == null) return;
    if (deepValueEquals(prevText, lines.value)) return; // already equal
    text.updateView(view, prevText, lines.value);
    updateLineRefs(view);
    prevText = lines.value;
  });

  // formatting
  function markFormatAction(mark: TextMarkType): ActionImplementation {
    return {
      isEnabled: () => toValue(isInput),
      isChecked: () => view != null && hasTextMark(view.state, view.state.selection, mark) !== false,
      action: () => {
        if (view == null) throw new Error("view not mounted");
        setTextMark(view.state, view.state.selection, mark, "toggle", view.dispatch);
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
    "text.format.code": markFormatAction("code"),
    "text.edit.hardBreak": {
      action: () => {
        // insert 'hardBreak' node at cursor
        if (view == null || toValue(suppressEnter)) return; // suppress if needed
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
      const { node: targetNode, pos: targetPos } = findLineNodeById(state, anchor.id!);
      if (targetNode == null) {
        selection = TextSelection.atEnd(state.doc);
      } else if (targetNode.type.name == "block") {
        selection = NodeSelection.create(state.doc, targetPos!);
      } else {
        selection = TextSelection.near(state.doc.resolve(targetPos!));
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
    tr.setMeta(plugin, { forceUpdate: Date.now() });
    view.dispatch(tr);
  }

  return { focus, actions, updatePlugin, lineRefsById };
}
