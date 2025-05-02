import { canvas, supergraph } from "@/globals";
import { NodeReferenceData, NodeType } from "@/proto/wire";
import { isNodeRef } from "@/proto/wiring";
import { startDragging } from "@/ui/drag";
import { pushDefaultMenu } from "@/ui/popover";
import { PM_SCHEMA } from "@/ui/prosemirror/schema";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { blurDocument } from "@/utils/element";
import { NavigationDirection, ViewExpose } from "@/views/common";
import { MaybeElement, useElementBounding } from "@vueuse/core";
import { Node as PmNode } from "prosemirror-model";
import {
  EditorState,
  NodeSelection,
  Plugin,
  PluginKey,
  PluginView,
  Transaction as PmTransaction,
  TextSelection,
} from "prosemirror-state";
import {
  Decoration,
  DecorationSet,
  EditorView,
  ViewMutationRecord,
  type NodeView as PmNodeView,
} from "prosemirror-view";
import {
  Component,
  ComponentInternalInstance,
  createVNode,
  markRaw,
  MaybeRef,
  reactive,
  Ref,
  render,
  toValue,
  VNode,
} from "vue";
import hljs from "highlight.js";

const PROSEMIRROR_NODE_KEY = "__pmNode";

/** Base Vue component renderer for PM NodeViews */
export class VueComponentView implements PmNodeView {
  dom: HTMLElement;
  component: Component;
  parentComponent: ComponentInternalInstance;
  props: Record<string, any>;
  pmnode: PmNode;
  vnode: VNode;
  view: EditorView;

  constructor(options: {
    style: "block" | "inline";
    component: Component;
    parentComponent: ComponentInternalInstance;
    props: Record<string, any>;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
  }) {
    const { style, component, parentComponent, props, node, view, getPos } = options;
    this.component = component;
    this.parentComponent = parentComponent;
    this.props = props;
    this.dom = document.createElement("div");
    if (style == "inline") {
      this.dom.style.display = "inline-block";
    }
    this.pmnode = node;
    this.vnode = this.createVNode();
    this.view = view;
    render(this.vnode, this.dom);
  }

  createVNode() {
    const vnode = createVNode(this.component, { ...this.props });
    vnode.appContext = this.parentComponent.appContext;
    (vnode as any).parent = this.parentComponent;
    (vnode as any)[PROSEMIRROR_NODE_KEY] = this.pmnode;
    return vnode;
  }

  render() {
    this.vnode = this.createVNode();
    render(this.vnode, this.dom);
  }

  update(node: PmNode, decorations: readonly Decoration[]): boolean {
    // don't need to update props?
    return true;
  }

  selectNode() {
    (this.vnode.component?.exposed as ViewExpose)?.focus?.("right");
  }

  stopEvent(event: Event): boolean {
    // forward events for node selections
    if (event instanceof KeyboardEvent && canvas.selection != null && canvas.selection.nodesPtr.length > 0) {
      const link = supergraph.getLinkMany(canvas.selection.nodesPtr);
      if (link != null) {
        return false;
      }
    }

    // suppress all other events
    return true;
  }

  ignoreMutation(mutation: ViewMutationRecord): boolean {
    // ignore all DOM mutations
    return true;
  }

  destroy() {
    render(null, this.dom);
  }
}

/** Mini-component for PM Node spans */
export class SpanNodeView extends VueComponentView {
  constructor(options: {
    component: Component;
    parentComponent: ComponentInternalInstance;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
  }) {
    const { component, parentComponent, node, view, getPos } = options;
    if (node.type.name != "spanMention") {
      throw new Error(`node is not 'span': ${node.type.name}`);
    }
    super({
      style: "inline",
      component,
      parentComponent,
      props: { nodePtr: node.attrs.nodePtr, isMinimal: true, isNested: true, size: "inherit" },
      node,
      view,
      getPos,
    });
    this.dom.classList.add("span-node-view");
  }

  updateNode() {}
}

/** BlockRenderer renders a custom Block vue component for 'block' nodes */
export class LineBlockView extends VueComponentView {
  navigateOuter: (direction: NavigationDirection) => void;
  blockPtr: NodeReferenceData;
  nodePtr: NodeReferenceData | undefined;
  getPos: () => number | undefined; // ensure we have access to getPos
  lineHandleDom: HTMLElement;

  constructor(options: {
    component: Component;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
    navigate: (direction: NavigationDirection) => void;
    parentComponent: ComponentInternalInstance;
  }) {
    const { component, node, view, getPos, parentComponent, navigate } = options;
    const props = {
      id: node.attrs.blockPtr.id,
      pageKey: parentComponent.uid.toString(),
      nodePtr: node.attrs.blockPtr,
      onNavigate: (direction: NavigationDirection) => this.navigate(direction),
    };
    super({
      style: "block",
      component,
      parentComponent,
      props,
      node,
      view,
      getPos,
    });
    this.navigateOuter = navigate;
    this.getPos = getPos;

    // view
    this.blockPtr = node.attrs.blockPtr;
    this.nodePtr = node.attrs.nodePtr;
    this.setupView(this.blockPtr, this.nodePtr);
    this.lineHandleDom = createLineHandleDom(node);
    this.dom.appendChild(this.lineHandleDom);
  }

  setupView(blockPtr: NodeReferenceData, nodePtr: NodeReferenceData | undefined) {
    this.dom.classList.remove(...this.dom.classList);
    this.dom.classList.add("line-block");
    const nodeType = nodePtr?.nodeType;
    if (nodeType != null) {
      const nodeTypeName = NodeType[nodeType].toLowerCase();
      this.dom.classList.add(nodeTypeName);
    }
    this.dom.dataset.nodeType = this.blockPtr.nodeType.toString();
    this.dom.dataset.nodeId = blockPtr.id;
    this.dom.dataset.nodeCk = blockPtr.ck;
  }

  /** Update the view when the node changes */
  update(node: PmNode, decorations: readonly Decoration[]): boolean {
    if (this.blockPtr?.id != node.attrs.blockPtr.id || this.nodePtr?.id != node.attrs.nodePtr.id) {
      // re-init
      this.blockPtr = node.attrs.blockPtr;
      this.nodePtr = node.attrs.nodePtr;
      this.props = { ...this.props, id: this.blockPtr.id, nodePtr: this.blockPtr };
      this.setupView(this.blockPtr, this.nodePtr);
      this.lineHandleDom.remove();
      this.lineHandleDom = createLineHandleDom(node);
      this.dom.appendChild(this.lineHandleDom);
      this.render();
    }
    return true;
  }

  /** Navigate to a sibling node (or fall back to outer navigation) */
  navigate(direction: NavigationDirection) {
    const pos = this.getPos();
    if (pos === undefined) {
      // we're lost
      this.navigateOuter(direction);
      return;
    }

    const { state, dispatch } = this.view;
    const $pos = state.doc.resolve(pos);
    const parent = $pos.parent;
    const index = $pos.index();

    const getChildPos = (childIndex: number): number => {
      let offset = 0;
      for (let i = 0; i < childIndex; i++) {
        offset += parent.child(i).nodeSize;
      }
      return $pos.start() + offset;
    };

    let targetPos: number | null = null;
    if (direction === "up" || direction === "left") {
      if (index > 0) {
        // select previous sibling
        targetPos = getChildPos(index - 1);
      }
    } else if (direction === "down" || direction === "right" || direction == "enter") {
      if (index < parent.childCount - 1 && direction != "enter") {
        // select next sibling
        targetPos = getChildPos(index + 1);
      } else {
        // create text line below
        this.view.dispatch(this.view.state.tr.insert(pos + 1, PM_SCHEMA.node("lineParagraph")));
        targetPos = pos + 1;
      }
    }

    if (targetPos !== null) {
      const targetNode = this.view.state.doc.nodeAt(targetPos);
      if (targetNode == null) throw new Error("targetNode is null");
      blurDocument(); // remove focus from current node
      let tr: PmTransaction;
      if (targetNode.type.name == "block") {
        tr = this.view.state.tr.setSelection(NodeSelection.create(this.view.state.doc, targetPos));
      } else {
        const $endPos = this.view.state.doc.resolve(targetPos);
        this.view.focus();
        tr = this.view.state.tr.setSelection(TextSelection.near($endPos));
      }
      this.view.dispatch(tr);
    } else {
      // use outer navigation, no sibling
      this.navigateOuter(direction);
    }
  }

  setSelection(anchor: number, head: number, root: Document | ShadowRoot) {
    // nothing to do?
  }

  destroy(): void {
    this.lineHandleDom.remove();
    super.destroy();
  }
}

/**
 * CodeLineView is our custom READONLY view for Code lines.
 */
export class CodeLineView implements PmNodeView {
  dom: HTMLElement;
  metaDom: HTMLElement;
  codeDom: HTMLElement | null;
  node: PmNode;
  view: EditorView;
  getPos: () => number | undefined;
  language: string | null;

  constructor(node: PmNode, view: EditorView, getPos: () => number | undefined) {
    this.node = node;
    this.view = view;
    this.getPos = getPos;
    this.language = node.attrs.language;

    // outer DOM element
    this.dom = document.createElement("code");
    this.dom.classList.add("line", "flex", "flex-col", "relative");
    if (this.language) {
      this.dom.classList.add(`language-${this.language}`);
    }

    // meta row
    this.metaDom = document.createElement("div");
    this.metaDom.classList.add(
      "flex",
      "flex-row",
      "justify-between",
      "w-full",
      "border-b",
      "select-none",
      "pb-1.5",
      "mb-2",
    );
    const languageDom = document.createElement("span");
    languageDom.classList.add("text-gray-400", "text-xs", "font-mono");
    languageDom.textContent = this.language ?? "code";
    this.metaDom.appendChild(languageDom);
    // meta controls
    const controlDom = document.createElement("div");
    controlDom.classList.add("flex", "flex-row", "gap-1");
    const copyButton = document.createElement("button");
    copyButton.classList.add(
      "text-gray-400",
      "transition-colors",
      "duration-150",
      "text-xs",
      "font-mono",
      "cursor-pointer",
      "hover:text-gray-700",
    );
    copyButton.addEventListener("click", () => {
      navigator.clipboard.writeText(node.textContent);
      copyIcon.classList.remove("fa-copy");
      copyIcon.classList.add("fa-check");
      setTimeout(() => {
        copyIcon.classList.remove("fa-check");
        copyIcon.classList.add("fa-copy");
      }, 2000);
    });
    const copyIcon = document.createElement("i");
    copyIcon.classList.add("fas", "fa-copy");
    copyButton.appendChild(copyIcon);
    controlDom.appendChild(copyButton);
    this.metaDom.appendChild(controlDom);
    this.dom.appendChild(this.metaDom);

    // main text content
    if ((node.textContent?.trim() ?? "").length > 0) {
      this.codeDom = document.createElement("pre");
      this.codeDom.textContent = node.textContent;
      this.dom.appendChild(this.codeDom);
    } else {
      const placeholderDom = document.createElement("div");
      placeholderDom.classList.add("text-gray-400", "font-mono");
      placeholderDom.textContent = " ";
      this.dom.appendChild(placeholderDom);
      this.codeDom = null;
    }

    // apply highlighting after the element is added to the DOM
    this.updateHighlighting();
  }

  update(node: PmNode) {
    this.node = node;
    if (node.type !== this.node.type) {
      return false;
    }

    // update highlighting
    this.updateHighlighting();

    return true;
  }

  updateHighlighting() {
    if (this.codeDom != null) {
      hljs.highlightElement(this.codeDom);
    }
  }
}

/**
 * Highlighting
 */

const highlightPluginKey = new PluginKey("highlightPlugin");
export function useHighlightPlugin(options: { selectedBlockIds: Ref<string[]>; draggingBlockIds: Ref<string[]> }) {
  const { selectedBlockIds, draggingBlockIds } = options;

  const plugin = new Plugin({
    key: highlightPluginKey,
    state: {
      init(_config, { doc }) {
        return DecorationSet.empty;
      },
      apply(tr, oldDecos, oldState, newState) {
        const decorations: Decoration[] = [];
        newState.doc.descendants((node, pos) => {
          if (node.attrs.blockPtr == null) return;
          const isSelected = selectedBlockIds.value.includes(node.attrs.blockPtr.id);
          const isDragging = draggingBlockIds.value.includes(node.attrs.blockPtr.id);
          if (isSelected || isDragging) {
            let className;
            if (isSelected) {
              if (isDragging) {
                className = "selected dragging";
              } else {
                className = "selected";
              }
            } else if (isDragging) {
              className = "dragging";
            }
            decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: className }));
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
  return plugin;
}

/**
 * Tooltip
 */

export type TooltipProps = {
  view: EditorView;
  visible: boolean;
  height: number;
  tick: number;
};

const TOOLTIP_SHOW_DELAY = 300;
const TOOLTIP_HIDE_DELAY = 500;
class TooltipPlugin implements PluginView {
  component: Component;
  parentComponent: ComponentInternalInstance;
  containerBounding: ReturnType<typeof useElementBounding>;
  gutterWidth: MaybeRef<number>;
  dom: HTMLElement;
  props: TooltipProps;
  vnode: VNode;
  tick = 0;
  tooltipTimer: ReturnType<typeof setTimeout> | null = null;
  hideTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(options: {
    view: EditorView;
    component: Component;
    parentComponent: ComponentInternalInstance;
    containerBounding: ReturnType<typeof useElementBounding>;
    gutterWidth?: MaybeRef<number>;
  }) {
    const { view, component, parentComponent, containerBounding, gutterWidth } = options;

    // state
    this.component = component;
    this.parentComponent = parentComponent;
    this.containerBounding = containerBounding;
    this.gutterWidth = gutterWidth ?? 0;

    // view
    this.dom = document.createElement("div");
    this.dom.addEventListener("mousedown", (event) => {
      event.stopPropagation();
      event.preventDefault();
    });
    view.dom.parentNode?.appendChild(this.dom);
    this.props = reactive({
      visible: false,
      view: markRaw(view),
      height: VIEW_DEFAULT_HEADER_HEIGHT,
      tick: this.tick,
    });
    this.vnode = this.createVNode();
    render(this.vnode, this.dom);
    this.update(view, null);
  }

  createVNode(): VNode {
    const vnode = createVNode(this.component, this.props);
    vnode.appContext = this.parentComponent.appContext;
    return vnode;
  }

  render() {
    this.vnode = this.createVNode();
    render(this.vnode, this.dom);
  }

  update(view: EditorView, lastState: EditorState | null) {
    this.tick++;
    this.props.tick = this.tick;
    this.props.view = view;

    // don't do anything if the document/selection didn't change
    const state = view.state;
    if (lastState && lastState.doc.eq(state.doc) && lastState.selection.eq(state.selection)) {
      return;
    }

    // clear any existing timers when selection changes
    if (this.tooltipTimer) {
      clearTimeout(this.tooltipTimer);
      this.tooltipTimer = null;
    }
    if (this.hideTimer) {
      clearTimeout(this.hideTimer);
      this.hideTimer = null;
    }

    // only show tooltip if selection is inside a line group
    const { $from, $to, empty } = state.selection;
    let hasText = false;
    state.doc.nodesBetween($from.pos, $to.pos, (node, pos) => {
      if (node.type.isText) {
        hasText = true;
      }
    });

    // update tooltip
    if (empty || !hasText) {
      // hide tooltip after delay
      this.hideTimer = setTimeout(() => {
        this.props.visible = false;
        this.render();
      }, TOOLTIP_HIDE_DELAY);
    } else {
      // show tooltip after delay
      this.updatePosition(view, state);
      this.tooltipTimer = setTimeout(() => {
        // reposition tooltip and update its content
        this.props.visible = true;
        this.updatePosition(view, state);
        this.render();
      }, TOOLTIP_SHOW_DELAY);
    }
    this.render();
  }

  updatePosition(view: EditorView, state: EditorState) {
    const fromPos = view.coordsAtPos(state.selection.from);
    this.dom.style.position = "fixed";
    this.dom.style.left = this.containerBounding.left.value + toValue(this.gutterWidth) + "px";
    this.dom.style.top = fromPos.top - this.props.height - 6 + "px";
  }

  destroy() {
    if (this.tooltipTimer) {
      clearTimeout(this.tooltipTimer);
    }
    if (this.hideTimer) {
      clearTimeout(this.hideTimer);
    }
    render(null, this.dom);
  }
}

export function useTooltipPlugin(options: {
  component: Component;
  parentComponent: ComponentInternalInstance;
  container: Ref<MaybeElement | null | undefined>;
  gutterWidth?: MaybeRef<number>;
}) {
  const { component, parentComponent, container, gutterWidth } = options;
  const containerBounding = useElementBounding(container);
  const plugin = new Plugin({
    view(editorView) {
      return new TooltipPlugin({ view: editorView, component, parentComponent, containerBounding, gutterWidth });
    },
  });
  return plugin;
}

/**
 * Line handles
 */

export const LINE_HANDLE_PLUGIN_KEY = new PluginKey("lineHandlePlugin");

let handleId = 0;

/** Create a DOM element for a line handle. */
function createLineHandleDom(node: PmNode): HTMLElement {
  const id = handleId++;
  const blockPtr = node.attrs.blockPtr as NodeReferenceData;

  function createButton(icon: string): HTMLElement {
    const button = document.createElement("button");
    button.className = "line-handle-button";
    const span = document.createElement("span");
    span.className = icon + " cursor-pointer";
    button.appendChild(span);
    return button;
  }

  const containerDom = document.createElement("div");
  containerDom.className = "line-handle";

  // drag
  const dragButton = createButton("fas fa-grip-vertical");
  dragButton.draggable = true;
  // data-suppress-drag="select"
  dragButton.dataset.suppressDrag = "select";
  dragButton.addEventListener("click", (event) => {
    if (!isNodeRef(blockPtr)) {
      throw new Error(`blockPtr is not a node ref from ${node.type.name}`);
    }
    pushDefaultMenu("context", blockPtr, event);
  });
  dragButton.addEventListener("dragstart", (event) => {
    if (!isNodeRef(blockPtr)) {
      throw new Error(`blockPtr is not a node ref from ${node.type.name}`);
    }
    startDragging(event, blockPtr);
    event.stopPropagation();
  });
  containerDom.appendChild(dragButton);

  return containerDom;
}

export function useLineHandlePlugin() {
  function buildLineHandleDecorations(doc: PmNode) {
    const decorations: Decoration[] = [];
    doc.descendants((node: PmNode, pos: number) => {
      // only create line handles for text nodes (block line handles are created in the LineBlockView)
      if (node.type.isInGroup("line") && !node.type.isInGroup("line-container") && node.type.name != "block") {
        const widget = Decoration.widget(pos + 1, () => createLineHandleDom(node), { side: 10 });
        decorations.push(widget);
      }
    });
    const decorationSet = DecorationSet.create(doc, decorations);
    return decorationSet;
  }

  const plugin = new Plugin({
    key: LINE_HANDLE_PLUGIN_KEY,
    state: {
      init(_, { doc }) {
        return buildLineHandleDecorations(doc);
      },
      apply(tr, decorationSet, oldState, newState) {
        if (tr.docChanged) {
          return buildLineHandleDecorations(tr.doc);
        }
        return decorationSet.map(tr.mapping, tr.doc);
      },
    },
    props: {
      decorations(state) {
        return this.getState(state);
      },
    },
  });
  return plugin;
}

/**
 * Placeholders
 */

export const PLACEHOLDER_PLUGIN_KEY = new PluginKey("placeholderPlugin");

export interface PlaceholderConfig {
  placeholderByNodeType: { [nodeType: string]: string };
  defaultPlaceholder: string | undefined;
}

const DEFAULT_PLACEHOLDER_CONFIG: PlaceholderConfig = {
  placeholderByNodeType: {
    lineListOrdered: "List",
    lineListUnordered: "List",
    lineQuote: "Quote",
    lineCallout: "Callout",
    lineCode: "Code",
    lineHeading: "Heading",
  },
  defaultPlaceholder: undefined,
};

/**
 * ProseMirror placeholder plugin.
 */
export function usePlaceholderPlugin(
  config: Partial<PlaceholderConfig> & {
    showIfUnfocused?: boolean;
    showAtBeginningOnly?: boolean;
    showIfEmpty?: string[];
  },
) {
  const {
    placeholderByNodeType,
    defaultPlaceholder,
    showIfUnfocused = false,
    showAtBeginningOnly = false,
    showIfEmpty = ["lineHeading"],
  } = {
    ...DEFAULT_PLACEHOLDER_CONFIG,
    ...config,
  };

  /** Create a placeholder widget element. */
  function createPlaceholderWidget(text: string, showIfUnfocused: boolean): HTMLElement {
    const span = document.createElement("span");
    span.className = "placeholder";
    if (!showIfUnfocused) {
      span.classList.add("placeholder-hidden");
    }
    span.textContent = text;
    return span;
  }

  /** Get placeholder text for a node if it should have one */
  function getPlaceholderMaybe(node: PmNode, isSelected: boolean): string | null {
    if (!isSelected && !showIfEmpty.includes(node.type.name)) {
      return null;
    }

    // only show for empty textblocks
    if (!node.isTextblock || node.textContent !== "") return null;
    let hasSpecialInput = false;
    node.descendants((child) => {
      if (child.type.name === "spanSpecialInput") {
        hasSpecialInput = true;
      }
    });
    if (hasSpecialInput) return null;

    // get placeholder text
    return placeholderByNodeType[node.type.name] || defaultPlaceholder || null;
  }

  /** Create a DecorationSet with placeholder widgets */
  function getPlaceholderDecorations(state: EditorState): DecorationSet {
    const decorations: Decoration[] = [];
    const { $from } = state.selection;
    const selectedParent = $from.parent;

    // add decorations for nodes that should have placeholders
    state.doc.descendants((node, pos) => {
      const isSelected = node === selectedParent && state.selection.empty;
      const placeholderText = getPlaceholderMaybe(node, isSelected);

      if (placeholderText) {
        const deco = Decoration.widget(
          pos + 1,
          () => createPlaceholderWidget(placeholderText, showIfUnfocused || showIfEmpty.includes(node.type.name)),
          {
            side: 1,
            ignoreSelection: true,
          },
        );
        decorations.push(deco);
      }
    });

    return DecorationSet.create(state.doc, decorations);
  }

  const plugin = new Plugin({
    key: PLACEHOLDER_PLUGIN_KEY,
    props: {
      decorations(state) {
        if (showAtBeginningOnly && state.doc.textContent.trim() !== "") {
          // don't show placeholders if the document is not fully empty
          return DecorationSet.create(state.doc, []);
        }
        return getPlaceholderDecorations(state);
      },
    },
  });
  return plugin;
}
