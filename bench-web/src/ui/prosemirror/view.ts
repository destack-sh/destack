import { supergraph } from "@/globals";
import { NodeReferenceData, NodeType, ObjectType } from "@/proto/wire";
import { DEFAULT_MISSING_ICON, getNodeIcon, getNodeName, ICON_BY_NODE_TYPE } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { Node as PmNode } from "prosemirror-model";
import { Decoration, EditorView, ViewMutationRecord, type NodeView as PmNodeView } from "prosemirror-view";
import { Component, ComponentInternalInstance, createVNode, render } from "vue";

/** Mini-component for PM Node spans */
export class SpanNodeView implements PmNodeView {
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
  vnode: any;
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
    this.vnode = createVNode(component, { ...props });
    this.vnode.parent = parentComponent;
    this.vnode.appContext = parentComponent.appContext;
    console.log("VueComponentRenderer", { ...options, vnode: this.vnode, dom: this.dom });
    render(this.vnode, this.dom);
  }
  update(node: PmNode, decorations: readonly Decoration[]): boolean {
    // don't need to update props?
    return true;
  }
  destroy() {
    render(null, this.dom);
  }
  ignoreMutation(mutation: ViewMutationRecord): boolean {
    return true;
  }
}

/** BlockRenderer renders a custom Block vue component for 'block' nodes */
export class BlockRenderer extends VueComponentRenderer {
  constructor(options: {
    component: Component;
    node: PmNode;
    view: EditorView;
    getPos: () => number | undefined;
    parentComponent: ComponentInternalInstance;
  }) {
    const { component, node, view, getPos, parentComponent } = options;
    super({
      component,
      parentComponent,
      props: {
        id: node.attrs.blockPtr.id,
        pageKey: parentComponent.uid.toString(),
        nodePtr: node.attrs.blockPtr,
      },
      node,
      view,
      getPos,
    });
    this.dom.classList.add("pm-block-view");
  }

  selectNode() {
    console.log("selectNode", this);
    this.vnode.focus?.("top");
  }
}
