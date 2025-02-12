import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { NodeReferenceData, NodeType, ObjectType } from "@/proto/wire";
import { isNodeRef } from "@/proto/wiring";
import { startDragging } from "@/ui/drag";
import { DEFAULT_MISSING_ICON, getNodeIcon, getNodeName, ICON_BY_NODE_TYPE } from "@/ui/icon";
import { pushDefaultContextMenu, pushDefaultMenu } from "@/ui/popover";
import { PM_SCHEMA } from "@/ui/prosemirror/schema";
import { getColorHex } from "@/ui/style";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { blurDocument } from "@/utils/element";
import { NavigationDirection, ViewExposed } from "@/views/common";
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

const PROSEMIRROR_NODE_KEY = "__pmNode";

/** Base Vue component renderer for PM NodeViews */
export class VueComponentView implements PmNodeView {
  dom: HTMLElement;
  pmnode: PmNode;
  vnode: VNode;
  view: EditorView;

  constructor(options: {
    component: Component;
    parentComponent: ComponentInternalInstance;
    props: Record<string, any>;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
  }) {
    const { component, parentComponent, props, node, view, getPos } = options;
    if (node.type.name != "block") {
      throw new Error(`node is not 'block': ${node.type.name}`);
    }
    this.dom = document.createElement("div");
    this.pmnode = node;
    this.vnode = createVNode(component, { ...props });
    this.vnode.appContext = parentComponent.appContext;
    (this.vnode as any).parent = parentComponent;
    (this.vnode as any)[PROSEMIRROR_NODE_KEY] = node;
    this.view = view;
    render(this.vnode, this.dom);
  }

  update(node: PmNode, decorations: readonly Decoration[]): boolean {
    // don't need to update props?
    return true;
  }

  destroy() {
    render(null, this.dom);
  }

  stopEvent(event: Event): boolean {
    // suppress all events
    return true;
  }

  ignoreMutation(mutation: ViewMutationRecord): boolean {
    // ignore all DOM mutations
    return true;
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
    if (node.type.name != "spanNode") {
      throw new Error(`node is not 'span': ${node.type.name}`);
    }
    super({
      component,
      parentComponent,
      props: { nodePtr: node.attrs.nodePtr, isMinimal: true },
      node,
      view,
      getPos,
    });
  }

  updateNode() {}
}

/** BlockRenderer renders a custom Block vue component for 'block' nodes */
export class LineBlockView extends VueComponentView {
  navigateOuter: (direction: NavigationDirection) => void;
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
      component,
      parentComponent,
      props,
      node,
      view,
      getPos,
    });
    this.navigateOuter = navigate;
    this.getPos = getPos;

    // meta
    this.dom.classList.add("line-block");
    const nodeType = node.attrs.nodePtr.nodeType;
    const nodeTypeName = NodeType[nodeType].toLowerCase();
    this.dom.classList.add(nodeTypeName);
    this.dom.dataset.nodeType = node.attrs.nodePtr.nodeType;
    this.dom.dataset.nodeId = node.attrs.nodePtr.id;
    this.dom.dataset.nodeCk = node.attrs.nodePtr.ck;

    // create line handle inside the block
    this.lineHandleDom = createLineHandleDom(node);
    this.dom.appendChild(this.lineHandleDom);
  }

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
      if (index < parent.childCount - 1) {
        // select next sibling
        targetPos = getChildPos(index + 1);
      } else {
        // create text line below
        this.view.dispatch(this.view.state.tr.insert(pos + 1, PM_SCHEMA.node("lineParagraph")));
        targetPos = pos + 2;
      }
    }

    if (targetPos !== null) {
      const targetNode = state.doc.nodeAt(targetPos);
      if (targetNode == null) throw new Error("targetNode is null");
      blurDocument(); // remove focus from current node
      let tr: PmTransaction;
      if (targetNode.type.name == "block") {
        tr = this.view.state.tr.setSelection(NodeSelection.create(this.view.state.doc, targetPos));
      } else {
        const $endPos = this.view.state.doc.resolve(targetPos);
        this.view.focus();
        tr = this.view.state.tr.setSelection(new TextSelection($endPos));
      }
      this.view.dispatch(tr);
    } else {
      // use outer navigation, no sibling
      this.navigateOuter(direction);
    }
  }

  selectNode() {
    (this.vnode.component?.exposed as ViewExposed)?.focus?.("top");
  }

  setSelection(anchor: number, head: number, root: Document | ShadowRoot) {
    // nothing to do?
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
        // recompute decorations based on the external highlightedBlockIds
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
    const { $from, empty } = state.selection;
    let isInsideLineGroup = false;
    for (let depth = $from.depth; depth >= 0; depth--) {
      if ($from.node(depth).type.isInGroup("line")) {
        isInsideLineGroup = true;
        break;
      }
    }

    // update tooltip
    if (empty || !isInsideLineGroup) {
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

/** Create a DOM element for a line handle. */
function createLineHandleDom(node: PmNode): HTMLElement {
  const blockPtr = node.attrs.blockPtr as NodeReferenceData;

  function createButton(icon: string): HTMLElement {
    const button = document.createElement("button");
    button.className = "line-handle-button";
    const span = document.createElement("span");
    span.className = icon;
    button.appendChild(span);
    return button;
  }

  const containerDom = document.createElement("div");
  containerDom.className = "line-handle";

  // add
  const addButton = createButton("fas fa-plus");
  addButton.addEventListener("click", (event) => {
    if (!isNodeRef(blockPtr)) {
      throw new Error(`blockPtr is not a node ref from ${node.type.name}`);
    }
    // nocheckin: add button
    event.stopPropagation();
    event.preventDefault();
  });
  containerDom.appendChild(addButton);

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
  function buildLineHandleDecorations(doc: any) {
    const decorations: Decoration[] = [];
    doc.descendants((node: PmNode, pos: number) => {
      // only create line handles for text nodes (block line handles are created in the BlockRenderer)
      if (node.type.isInGroup("line") && node.type.name != "block") {
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
export function usePlaceholderPlugin(config: Partial<PlaceholderConfig>) {
  const { placeholderByNodeType, defaultPlaceholder } = { ...DEFAULT_PLACEHOLDER_CONFIG, ...config };

  /** Create a placeholder widget element. */
  function createPlaceholderWidget(text: string): HTMLElement {
    const span = document.createElement("span");
    span.className = "placeholder";
    span.textContent = text;
    return span;
  }

  /** Create a DecorationSet with a placeholder widget for an empty textblock containing the selection. */
  function getPlaceholderDecoration(state: EditorState): DecorationSet {
    const { $from } = state.selection;
    const parent = $from.parent;
    // only show placeholder if selection is inside an empty textblock
    if (!(parent.isTextblock && state.selection.empty && parent.textContent == "")) {
      return DecorationSet.empty;
    }

    // figure out placeholder text
    const placeholderText = placeholderByNodeType[parent.type.name] || defaultPlaceholder;
    if (placeholderText == null) {
      return DecorationSet.empty;
    }

    // insert widget at the beginning of the textblock.
    const pos = $from.start();
    const deco = Decoration.widget(pos, () => createPlaceholderWidget(placeholderText), {
      side: 1,
      ignoreSelection: true,
    });
    return DecorationSet.create(state.doc, [deco]);
  }

  const plugin = new Plugin({
    key: PLACEHOLDER_PLUGIN_KEY,
    props: {
      decorations(state) {
        return getPlaceholderDecoration(state);
      },
    },
  });
  return plugin;
}
