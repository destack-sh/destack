import { canvas, supergraph } from "@/globals";
import { newChangeId } from "@/language/core/transaction";
import { NodeReferenceData, TextLineType, TextSpanType } from "@/proto/wire";
import { type CommandKit, type CommandMapKit } from "@/ui/command";
import { PageContext } from "@/ui/prosemirror/page";
import { getPmLineType, PM_SCHEMA, SpanSpecialInputType, TextMarkType } from "@/ui/prosemirror/schema";
import { LineBlockView, SpanNodeView, VueComponentView, CodeLineView } from "@/ui/prosemirror/view";
import {
  LineInterface,
  mapLineToPmNode,
  mapPmNodeToText,
  mapTextToPmNode,
  TextInterface,
} from "@/ui/prosemirror/wiring";
import { deleteSelection } from "@/ui/space";
import { deepValueEquals } from "@/utils/ref";
import NodeReference from "@/views/builtin/NodeReference.vue";
import TextSpecialInput from "@/views/builtin/TextSpecialInput.vue";
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
import { EditorView, NodeViewConstructor } from "prosemirror-view";
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

/** Autowraps and unwraps any wrapable nodes. */
export function autoWrap(tr: PmTransaction) {
  const { doc } = tr;

  //
  // Wrap unwrapped list items
  //

  const children: Array<{ node: PmNode; pos: number; end: number }> = [];
  doc.forEach((node, pos) => {
    children.push({ node, pos, end: pos + node.nodeSize });
  });

  // find contiguous groups of unwrapped list items (of the same kind)
  interface UnwrappedGroup {
    start: number;
    end: number;
    containerType: PmNode["type"];
  }
  const groups: UnwrappedGroup[] = [];
  let i = 0;
  while (i < children.length) {
    const { node, pos, end } = children[i];
    if (node.type === PM_SCHEMA.nodes.lineListOrdered || node.type === PM_SCHEMA.nodes.lineListUnordered) {
      // determine container type based on list item type
      const containerType =
        node.type === PM_SCHEMA.nodes.lineListOrdered ? PM_SCHEMA.nodes.orderedList : PM_SCHEMA.nodes.unorderedList;
      const groupStart = pos;
      let groupEnd = end;
      let j = i + 1;
      while (j < children.length && children[j].node.type === node.type) {
        groupEnd = children[j].end;
        j++;
      }
      groups.push({ start: groupStart, end: groupEnd, containerType });
      i = j;
    } else {
      i++;
    }
  }

  // wrap each group in its appropriate container
  //  (in reverse order so that later changes don't affect earlier positions)
  for (let g = groups.length - 1; g >= 0; g--) {
    const group = groups[g];
    const $start = tr.doc.resolve(group.start);
    const $end = tr.doc.resolve(group.end);
    const range = $start.blockRange($end);
    if (range) {
      tr.wrap(range, [{ type: group.containerType }]);
    }
  }

  //
  // Merge adjacent list containers of the same type
  //

  const newChildren: Array<{ node: PmNode; pos: number; end: number }> = [];
  tr.doc.forEach((node, pos) => {
    newChildren.push({ node, pos, end: pos + node.nodeSize });
  });
  // iterate in reverse; if two consecutive nodes are list containers of the same type, join them
  for (let i = newChildren.length - 1; i > 0; i--) {
    const cur = newChildren[i];
    const prev = newChildren[i - 1];
    if (
      (cur.node.type === PM_SCHEMA.nodes.orderedList || cur.node.type === PM_SCHEMA.nodes.unorderedList) &&
      cur.node.type === prev.node.type
    ) {
      // the join position is at the start of the current container
      tr.join(cur.pos);
    }
  }

  //
  // Remove empty list containers
  //

  const finalChildren: Array<{ node: PmNode; pos: number; end: number }> = [];
  tr.doc.forEach((node, pos) => {
    finalChildren.push({ node, pos, end: pos + node.nodeSize });
  });
  for (let i = finalChildren.length - 1; i >= 0; i--) {
    const { node, pos } = finalChildren[i];
    if (
      (node.type === PM_SCHEMA.nodes.orderedList || node.type === PM_SCHEMA.nodes.unorderedList) &&
      node.childCount === 0
    ) {
      tr.delete(pos, pos + node.nodeSize);
    }
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
  const tr = state.tr;

  // map target node type
  const targetNodeType = getPmLineType(targetType);

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
    autoWrap(tr);
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
    autoWrap(tr);
    return tr;
  });
  return rule;
}

/** Create a line divider from --- or —- syntax */
const lineDividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.delete(start, end);
  tr.insert(start - 1, PM_SCHEMA.node("lineDivider"));
  tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
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
  if (marker.length === 1 || marker.length === 2) {
    regex = new RegExp(`(?<!${char})${escapedMarker}(?!${char})(.+?)${escapedMarker}(?!${char})$`);
  } else {
    throw new Error(`unsupported marker: ${marker}`);
  }
  return regex;
}

/** Create an input rule that transforms text wrapped in markers into text with a given mark. */
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

/** Create a link span from plain URL syntax */
const URL_PATTERN = /(?:(?:https?:\/\/))?(?:www\.)?(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}(?::\d{1,5})?(?:\/\S*)?\s+$/;
const urlLinkRule = new InputRule(URL_PATTERN, (state, match, start, end) => {
  const { tr } = state;
  // only apply if there isn't already a link here
  let hasLink = false;
  tr.doc.nodesBetween(start, end, (node, pos) => {
    if (node.type.name === "spanLink" || node.type.name === "spanCitation") {
      hasLink = true;
    }
  });
  if (hasLink) return null;

  let [url] = match;
  url = url.trim();
  const node = PM_SCHEMA.nodes.spanLink.create({ url }, PM_SCHEMA.text(url));
  tr.replaceWith(start, end, [node, PM_SCHEMA.text(" ")]);
  tr.setSelection(TextSelection.near(tr.doc.resolve(start + node.nodeSize + 1)));
  return tr;
});

/** Create a link span from [content](url) syntax */
const mdLinkRule = new InputRule(/\[(.+?)\]\((.+?)\)$/, (state, match, start, end) => {
  const { tr } = state;
  let [, text, url = ""] = match;
  text = text.trim();
  url = url.trim();
  if (text.length === 0) return null;
  const node = PM_SCHEMA.nodes.spanLink.create({ url }, PM_SCHEMA.text(text));
  tr.replaceWith(start, end, [node, PM_SCHEMA.text(" ")]);
  tr.setSelection(TextSelection.near(tr.doc.resolve(start + node.nodeSize + 1)));
  return tr;
});

/** Create a citation span from [^content] or [^content](url) syntax */
const mdCitationRule = new InputRule(/\[\^(.+?)\](?:\((.+?)\))?\s+$/, (state, match, start, end) => {
  const { tr } = state;
  let [, text, url = ""] = match;
  text = text.trim();
  url = url.trim();
  if (text.length === 0) return null;
  const node = PM_SCHEMA.nodes.spanCitation.create({ url }, PM_SCHEMA.text(text));
  tr.replaceWith(start, end, [node, PM_SCHEMA.text(" ")]);
  const newPos = tr.mapping.map(end);
  tr.setSelection(TextSelection.near(tr.doc.resolve(newPos)));
  return tr;
});

const UNORDERED_LIST_CHARS = ["-", "\\*", "•"];

const PM_BASIC_INPUT_RULES: InputRule[] = [
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
  markerRule("`", PM_SCHEMA.marks.code),
];

const PM_LINE_INPUT_RULES: InputRule[] = [
  ...PM_BASIC_INPUT_RULES,
  // special input rules
  specialInputRule(/@/, "@", "@"),
];

const PM_BLOCK_INPUT_RULES: InputRule[] = [
  ...PM_BASIC_INPUT_RULES,
  // special input rules
  specialInputRule(/^\/$/, "/", undefined),
  specialInputRule(/@/, "@", "@"),
  // link and citation rules
  mdLinkRule,
  urlLinkRule,
  mdCitationRule,
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
  linePrefixRule(/^[0-9]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule(/^\s[0-9]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule("> ", PM_SCHEMA.nodes.lineQuote, TextLineType.QUOTE),
  linePrefixRule("! ", PM_SCHEMA.nodes.lineCallout, TextLineType.CALLOUT),
  linePrefixRule("``` ", PM_SCHEMA.nodes.lineCode, TextLineType.CODE),
];

/** Build the ProseMirror commands. */
function getPmCommands(options: {
  mode: "line" | "block";
  navigate: (direction: NavigationDirection) => void;
  deleteSelf: () => void;
}) {
  const { mode, navigate, deleteSelf } = options;
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
      const atStart = empty && $from.parentOffset === 0 && $from.start() === $from.end() && $to.pos === $from.end();

      // delete node selection if we have one
      if (empty && canvas.selection != null && canvas.selection.nodesPtr.length > 0) {
        const link = supergraph.getLinkMany(canvas.selection.nodesPtr);
        if (link != null) {
          const tx = link.connection.tx.with({ change: { key: newChangeId(), title: "Delete" } });
          for (const node of link.nodes) {
            tx.delete(node);
          }
          return true;
        }
      }

      if (mode == "block" && atStart && $from.parent.type.name !== "lineParagraph") {
        // morph to plain paragraph
        return morphLineNode(state, $from, TextLineType.PARAGRAPH, dispatch);
      } else if (
        mode == "block" &&
        atStart &&
        $from.parent.type.name === "lineParagraph" &&
        $from.index(-1) > 0 &&
        $from.node(-1).child($from.index(-1) - 1).type.name == "block"
      ) {
        // delete this line and focus block above (can't delete blocks with backspace)
        let tr = state.tr;
        tr = tr.deleteRange($from.pos - 1, $to.end());
        tr = tr.setSelection(NodeSelection.create(tr.doc, $from.pos - 2));
        dispatch?.(tr);
        return true;
      } else if (empty && state.doc.textContent === "" && state.doc.children.length <= 1) {
        // delete self if doc is empty
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
      if (commands.exitCode(state, dispatch, view)) {
        return true;
      } else {
        return commands.splitBlockAs((node, atEnd, $from) => {
          if (node.type.isInGroup("line")) {
            // remove blockPtr from the new block (force create)
            return { type: node.type, attrs: { ...node.attrs, blockPtr: null } };
          }
          return null;
        })(state, dispatch);
      }
    },
    "Shift-Enter": (state, dispatch, view) => {
      if (commands.newlineInCode(state, dispatch, view)) {
        return true;
      } else {
        // insert spanHardBreak at cursor, but only if not at the beginning of the line
        const { from, $from } = state.selection;
        const atLineStart = $from.parentOffset === 0;
        if (!atLineStart) {
          const hardBreak = PM_SCHEMA.node("spanHardBreak");
          dispatch?.(state.tr.insert(from, hardBreak));
          return true;
        }
        return false;
      }
    },
    Enter(state, dispatch, view) {
      const { $from, $to } = state.selection;
      const parentType = $from.parent.type.name;
      if (parentType === "lineListOrdered" || parentType === "lineListUnordered" || parentType === "lineHeading") {
        if ($from.parent.textContent.trim() === "") {
          // morph to plain paragraph
          return morphLineNode(state, $from, TextLineType.PARAGRAPH, dispatch);
        } else {
          // otherwise, split at the cursor position (keep attrs except blockPtr)
          const tr = state.tr.split($from.pos, 1, [
            { type: $from.parent.type, attrs: { ...$from.parent.attrs, blockPtr: null } },
          ]);
          dispatch?.(tr);
          return true;
        }
      }
      if (commands.newlineInCode(state, dispatch, view)) {
        return true;
      } else {
        return commands.splitBlockAs((node, atEnd, $from) => {
          if (node.type.isInGroup("line")) {
            // remove blockPtr from the new block (force create)
            return { type: node.type, attrs: { ...node.attrs, blockPtr: null } };
          }
          return null;
        })(state, dispatch);
      }
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

/** Gets our custom ProseMirror format command */
export function markFormatCommand(
  mark: TextMarkType,
  getView: () => EditorView | null,
  isEnabled: () => boolean,
) {
  return {
    isEnabled: () => {
      const view = getView();
      if (view == null) return false;
      return isEnabled();
    },
    isChecked: () => {
      const view = getView();
      if (view == null) return false;
      return hasTextMark(view.state, view.state.selection, mark) !== false;
    },
    command: () => {
      const view = getView();
      if (view == null) throw new Error("view not mounted");

      if (mark == "code") {
        const { state } = view;
        const { selection } = state;

        // if we have a multi-line selection, convert to a code block
        if (selection instanceof TextSelection && !selection.empty) {
          const { $from, $to } = selection;
          const fromBlock = $from.blockRange();
          const toBlock = $to.blockRange();

          // multi-block selection or selection containing multiple lines
          if (
            (fromBlock && toBlock && fromBlock.start !== toBlock.start) ||
            state.doc.textBetween($from.pos, $to.pos, "\n").includes("\n")
          ) {
            const tr = state.tr;
            const selectedText = state.doc.textBetween($from.pos, $to.pos, "\n");
            const $start = state.doc.resolve($from.start($from.depth));
            const parentAttrs = $start.parent.attrs;

            // replace the selection with a code block
            const codeBlock = PM_SCHEMA.node(
              "lineCode",
              {
                type: TextLineType.CODE,
                blockPtr: parentAttrs.blockPtr,
              },
              state.schema.text(selectedText),
            );
            tr.replaceWith($from.start($from.depth), $to.end($to.depth), codeBlock);
            const newPos = $from.start($from.depth) + codeBlock.nodeSize - 1;
            tr.setSelection(TextSelection.create(tr.doc, newPos));
            view.dispatch(tr);
            return true;
          }
        }

        // otherwise, just toggle the code mark
        commands.toggleMark(state.schema.marks.code)(state, view.dispatch);
        return true;
      }
      setTextMark(view.state, view.state.selection, mark, "toggle", view.dispatch);
    },
  };
}

/**
 * Install a Text editor on a DOM element.
 */
export function useTextEditor(options: {
  mode: "line" | "block";
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
  onPmTransaction?: (view: EditorView, prevState: EditorState, newState: EditorState) => void;
}) {
  const { mode, textRef, text, isInput, suppressEnter, suppressDrop, navigate, deleteSelf } = options;

  // setup
  const bindings: Record<string, Command> = {
    ...commands.baseKeymap,
    ...getPmCommands({ mode, navigate, deleteSelf }),
  };
  if (mode == "line") {
    bindings["Shift-Enter"] = () => true;
    bindings["Mod-Enter"] = () => true;
    bindings.Enter = () => true;
  } else if (toValue(suppressEnter)) {
    bindings["Shift-Enter"] = bindings.Enter;
    bindings["Mod-Enter"] = () => true;
    bindings.Enter = () => true;
  }
  if (options?.history) {
    bindings["Mod-z"] = undo;
    bindings["Mod-y"] = redo;
    bindings["Mod-Shift-z"] = redo;
  }
  const plugins: Plugin[] = [
    keymap(bindings),
    inputRules({ rules: mode == "line" ? PM_LINE_INPUT_RULES : PM_BLOCK_INPUT_RULES }),
  ];
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
    const nodeViews: Record<string, NodeViewConstructor> = {
      spanMention: (node, view, getPos) =>
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
    };
    if (!toValue(isInput)) {
      nodeViews.lineCode = (node, view, getPos) => new CodeLineView(node, view, getPos);
    }
    const view = new EditorView(textRef.value, {
      state: makeEditorState(lines.value),
      editable: () => toValue(isInput),
      nodeViews,
      plugins,
      dispatchTransaction(tx) {
        if (view == null) throw new Error("view not mounted");
        // update the state
        const prevState = view.state;
        let newState = view.state.apply(tx);
        // also write the doc (if changed)
        if (tx.docChanged && !tx.getMeta("_ignoreDocChanged")) {
          const { lines: updatedText, linesNodes, linesPos } = mapPmNodeToText(newState.doc);
          prevText = text.write(updatedText);
          if (mode == "line") {
            // ensure line type stays the same
            let tr = newState.tr;
            const prevLine = prevText[0];
            const updatedLine = updatedText[0];
            if (prevLine.type == "text" && updatedLine.type == "text" && prevLine.text.type != updatedLine.text.type) {
              const prevLineNode = mapLineToPmNode(prevLine);
              tr = tr.replaceRangeWith(linesPos[0], linesPos[0] + prevLineNode.nodeSize, prevLineNode);
            }
            newState = newState.apply(tr);
          } else if (mode == "block" && prevText !== updatedText) {
            // sync back updated lines (may change during write)
            // NOTE :Cleanup: can't we just use differenceUpdateLines here?
            let tr = newState.tr;
            for (let i = 0; i < linesNodes.length; i++) {
              const lineNode = linesNodes[i];
              const line = prevText[i];
              if (line.blockPtr?.id !== lineNode.attrs.blockPtr?.id) {
                tr = tr.setNodeAttribute(linesPos[i], "blockPtr", line.blockPtr);
              }
              if (line.type == "block" && line.nodePtr?.id !== lineNode.attrs.nodePtr?.id) {
                tr = tr.setNodeAttribute(linesPos[i], "nodePtr", line.nodePtr);
              }
            }
            newState = newState.apply(tr);
          }
        }
        view.updateState(newState);
        if (tx.docChanged) {
          updateLineRefs(view);
        }
        options.onPmTransaction?.(view, prevState, newState);
      },
    });
    updateLineRefs(view);
    return view;
  }

  // mount the editor view
  whenever(textRef, () => {
    if (view) {
      view.destroy();
    }
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

  // actions
  const actions: CommandMapKit<"text"> & Partial<CommandMapKit<"space">> = {
    // text
    "text.format.bold": markFormatCommand(
      "bold",
      () => view,
      () => toValue(isInput),
    ),
    "text.format.italic": markFormatCommand(
      "italic",
      () => view,
      () => toValue(isInput),
    ),
    "text.format.strikethrough": markFormatCommand(
      "strikethrough",
      () => view,
      () => toValue(isInput),
    ),
    "text.format.underline": markFormatCommand(
      "underline",
      () => view,
      () => toValue(isInput),
    ),
    "text.format.code": markFormatCommand(
      "code",
      () => view,
      () => toValue(isInput),
    ),
    "text.format.spoiler": markFormatCommand(
      "spoiler",
      () => view,
      () => toValue(isInput),
    ),
    // space
    "space.edit.delete": {
      command: (command, ctx) => {
        if (deleteSelection(command, ctx)) {
          return true;
        }
      },
    },
    "space.select.all": {
      command: () => {
        return commands.selectAll(view!.state, view!.dispatch);
      },
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
    tr.setMeta(plugin, { _forceUpdate: Date.now() });
    view.dispatch(tr);
  }

  return { focus, actions, updatePlugin, lineRefsById };
}
