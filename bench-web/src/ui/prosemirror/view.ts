import { supergraph } from "@/globals";
import { NodeReferenceData, NodeType, ObjectType } from "@/proto/wire";
import { DEFAULT_MISSING_ICON, getNodeIcon, getNodeName, ICON_BY_NODE_TYPE } from "@/ui/icon";
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
  VNode
} from "vue";

const PROSEMIRROR_NODE_KEY = "__pmNode";

/** Mini-component for PM Node spans */
export class SpanNodeView implements PmNodeView {
  // nocheckin: wrap NodeReference in SpanNodeView? (instead of this custom stuff)
  dom: HTMLElement;
  nodePtr: NodeReferenceData;
  iconDom: HTMLElement;
  nameDom: HTMLElement;

  constructor(pmNode: PmNode, view: EditorView) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("spanNode");
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
      ...(ICON_BY_NODE_TYPE[pmNode.attrs.nodePtr.nodeType as unknown as NodeType]?.faName?.split(" ") ?? [
        "fas",
        "fa-question",
      ]),
    );
    this.nameDom = this.dom.appendChild(document.createElement("span"));
    this.nameDom.classList.add("name");
    this.nameDom.textContent = "???";

    this.updateNode();
  }

  updateNode() {
    const nodePtr = this.nodePtr;
    const node = supergraph.get(nodePtr);
    this.nameDom.textContent = (node != null ? getNodeName(node) : null) ?? "???";
    const icon = (node != null ? getNodeIcon(node) : null) ?? DEFAULT_MISSING_ICON;
    this.iconDom.className = icon?.faName != null ? `icon ${icon.faName}` : "icon fa fa-question";
    if (icon.color != null) this.iconDom.style.color = getColorHex(icon.color)!;
    else this.iconDom.style.removeProperty("color");
    this.dom.dataset.nodeType = nodePtr.nodeType.toString();
  }
}

/** Base Vue component renderer for PM NodeViews */
export class VueComponentRenderer implements PmNodeView {
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

/** BlockRenderer renders a custom Block vue component for 'block' nodes */
export class BlockRenderer extends VueComponentRenderer {
  navigateOuter: (direction: NavigationDirection) => void;
  getPos: () => number | undefined; // ensure we have access to getPos

  constructor(options: {
    component: Component;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
    navigate: (direction: NavigationDirection) => void;
    parentComponent: ComponentInternalInstance;
  }) {
    const { component, node, view, getPos, parentComponent, navigate } = options;
    // Pass the onNavigate prop so that the Vue component can trigger navigation.
    super({
      component,
      parentComponent,
      props: {
        id: node.attrs.blockPtr.id,
        pageKey: parentComponent.uid.toString(),
        nodePtr: node.attrs.blockPtr,
        onNavigate: (direction: NavigationDirection) => this.navigate(direction),
      },
      node,
      view,
      getPos,
    });
    this.navigateOuter = navigate;
    this.getPos = getPos;
    this.dom.classList.add("pm-block-view");
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
export function useHighlightPlugin(options: { selectedBlockIds: Ref<string[]> }) {
  const { selectedBlockIds } = options;

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

class TooltipPlugin implements PluginView {
  component: Component;
  parentComponent: ComponentInternalInstance;
  containerBounding: ReturnType<typeof useElementBounding>;
  gutterWidth: MaybeRef<number>;
  dom: HTMLElement;
  props: TooltipProps;
  vnode: VNode;
  tick = 0;

  constructor(options: {
    view: EditorView;
    component: Component;
    parentComponent: ComponentInternalInstance;
    containerBounding: ReturnType<typeof useElementBounding>;
    gutterWidth?: MaybeRef<number>;
  }) {
    const { view, component, parentComponent, containerBounding, gutterWidth } = options;
    this.component = component;
    this.parentComponent = parentComponent;
    this.containerBounding = containerBounding;
    this.gutterWidth = gutterWidth ?? 0;

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

  update(view: EditorView, lastState: EditorState | null) {
    this.tick++;
    this.props.tick = this.tick;
    this.props.view = view;

    // don't do anything if the document/selection didn't change
    const state = view.state;
    if (lastState && lastState.doc.eq(state.doc) && lastState.selection.eq(state.selection)) {
      return;
    }

    // update the tooltip
    if (state.selection.empty) {
      // hide the tooltip if the selection is empty
      this.props.visible = false;
    } else {
      // reposition tooltip and update its content
      this.props.visible = true;
      const { from } = state.selection;
      const fromPos = view.coordsAtPos(from);
      this.dom.style.position = "fixed";
      this.dom.style.left = this.containerBounding.left.value + toValue(this.gutterWidth) + "px";
      this.dom.style.top = fromPos.top - this.props.height - 6 + "px";
    }

    this.vnode = this.createVNode();
    render(this.vnode, this.dom);
  }

  destroy() {
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
