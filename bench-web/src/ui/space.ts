import { canvas, supergraph } from "@/globals";
import { BENCH_BENCH_AGENT_PTR } from "@/language/core/builtin";
import { getBaseFromNode, HELPER_VIEW_TYPES, isInlineNode, ROOT_VIEW_TYPES, toCamelName } from "@/language/core/const";
import { isDescendantOf, type NodeKey, type ReadNodeGraph } from "@/language/core/graph";
import {
  cloneNode,
  cloneNodes,
  generateNodeName,
  makeNode,
  NodeIn,
  packSubnode,
  unpackSubnode,
} from "@/language/core/node";
import { getOrderKey, updateOrder } from "@/language/core/order";
import {
  makeEdit,
  makeEditFromSubnode,
  newChangeId,
  TransactionOptions,
  type Transaction,
} from "@/language/core/transaction";
import { unpackBuiltinObject } from "@/language/core/value";
import { createPage } from "@/language/source/page";
import { createThread } from "@/language/source/thread";
import {
  BlockType,
  ChangeCategory,
  NodeMode,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  SelectionData,
  SpaceData,
  StructType,
  ViewData,
  ViewProperty,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import {
  describeNode,
  describeScope,
  isNode,
  isNodeRef,
  isStruct,
  makeStruct,
  propertyInfo,
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { benchConnection, benchGraph, inspectionPtr, pkg, space } from "@/system/space";
import { Command, CommandContext, declareCommands, getNodesForCommand, NON_DELETABLE_NODE_TYPES } from "@/ui/command";
import type { SplitAnchor } from "@/ui/drag";
import { getContainingFlow } from "@/ui/flow";
import { getNodeIcon, getNodeTitle, toIconMaybe } from "@/ui/icon";
import { DEFAULT_ORIENTATION, splitBox } from "@/ui/layout";
import { PopoverInfoIn, PopoverInstance, pushPopover } from "@/ui/popover";
import { toaster } from "@/ui/toast";
import {
  collectViewComponentsUp,
  findViewComponentUp,
  getViewComponentId,
  getViewComponentPtrMaybe,
  getVueComponentType,
  isIdentifiedViewComponent,
  isOutsideView,
  isViewComponent,
  isViewComponentIn,
  makeSelection,
  makeSelectionMaybe,
} from "@/ui/view";
import { isFocusableElement } from "@/utils/element";
import { generateOrderKey, generateOrderKeys } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { DISCORD_URL } from "@/utils/globals";
import { log } from "@/utils/log";
import { deepValueEquals } from "@/utils/ref";
import { Casing, toCasing } from "@/utils/string";
import { type FocusAnchor, type ViewComponent, type ViewProps } from "@/views/common";
import { useActiveElement, useEventListener } from "@vueuse/core";
import {
  computed,
  getCurrentInstance,
  nextTick,
  onBeforeUnmount,
  onMounted,
  onUpdated,
  ref,
  shallowRef,
  triggerRef,
  watch,
  type ComponentInstance,
  type Ref,
} from "vue";

export type SomeView = NodeReferenceData | ViewData;
export type ViewIn = Partial<NodeIn<NodeType.VIEW>> & { type: ViewType };

type OpenViewOptions = {
  filter?: (view: ViewData) => boolean;
  ifPresent?: "duplicate" | "focus" | "upsertAndFocus";
  props?: ViewIn;
};

export const NODE_REF_CONTEXT_KEY = Symbol("nodeRefContext");
export const activeElement = useActiveElement();

const SUBVIEWS_PROPERTY_ID = propertyInfo(ObjectType.VIEW, ViewProperty.subviewsPacked).id;
const SUBVIEWS_PROPERTY_KEY = SUBVIEWS_PROPERTY_ID.toString();

/**
 * Canvas for Views and their components in a Space.
 * Some of our View components may not have an associated View, so we track them with a derived id.
 * NOTE: SpaceCanvas is effectively a global singleton (currently).
 */
export class SpaceCanvas {
  spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
  space: Ref<SpaceData | null>;
  graph: ReadNodeGraph;
  tx: () => Transaction; // for when we're not given a transaction to work with (e.g. browser events)
  viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // :ViewComponentId

  // absolutely focused views/components (focused from the top down)
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up
  focusedViewPtr: Ref<TypedNodeReferenceData<NodeType.VIEW> | null> = shallowRef(null);

  // selection
  highlightedPtr = ref<NodeReferenceData | null>(null);
  selectedPtrById: Ref<Record<string, NodeReferenceData | null>> = shallowRef({});

  constructor(
    spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>,
    graph: ReadNodeGraph,
    txFactory: () => Transaction,
  ) {
    this.spacePtr = spacePtr;
    this.graph = graph;
    this.space = graph.getRef(spacePtr);
    this.tx = txFactory;

    // respond to 'unmanaged' input from browser
    // active element
    watch(activeElement, () => console.log("activeElement", activeElement.value));
    watch(activeElement, () => {
      if (
        activeElement.value != null &&
        activeElement.value !== document.body &&
        // NOTE: Chromium considers scrollable containers to be focusable elements, so ignore those.
        //  (Otherwise we would get confused because activeElement change comes after mousedown event,
        //   and if the mousedown'ed target was the next higher container it will trigger later and change focus)
        isFocusableElement(activeElement.value) &&
        !isOutsideView(activeElement.value)
      ) {
        this.onComponentFocused(activeElement.value);
      }
    });
    // and 'focus' on any other element
    useEventListener(document, "mousedown", (e) => {
      if (
        e.target != null &&
        e.target != activeElement.value &&
        !(e.target as HTMLElement).hasAttribute("data-v-app") &&
        !isOutsideView(e.target as HTMLElement)
      ) {
        this.onComponentFocused(e.target as HTMLElement);
      }
    });
    // track currently hovered node for highlighting
    useEventListener(document, "mousemove", (e) => {
      const nodePtr = this.getNodeAt(e.target as HTMLElement);
      if (nodePtr?.id != this.highlightedPtr.value?.id) {
        this.highlightedPtr.value = nodePtr;
      }
    });
    // inspect node on double click (sometimes single click is overridden by other events)
    useEventListener(document, "dblclick", (e) => {
      const nodePtr = this.getNodeAt(e.target as HTMLElement);
      if (nodePtr != null && this.inspection?.id != nodePtr.id) {
        this.inspect({ node: nodePtr });
      }
    });

    // update selection with space selection
    watch(
      () => space.value?.selection,
      () => {
        if (space.value?.selection != null) {
          this.selectedPtrById.value = space.value.selection.nodesPtr.reduce(
            (acc, nodePtr) => {
              acc[nodePtr.id!] = nodePtr;
              return acc;
            },
            {} as Record<string, NodeReferenceData | null>,
          );
        } else {
          this.selectedPtrById.value = {};
        }
      },
    );
  }

  /** Whether the node is currently inspected. */
  isInspected(node: NodeKey<any> | null | undefined): boolean {
    return (
      node != null &&
      inspectionPtr.value != null &&
      (inspectionPtr.value.id == node.id || inspectionPtr.value.ck == (node as any).ck)
    );
  }

  /** Whether the node is currently highlighted. */
  isHighlighted(node: NodeKey<any> | null | undefined): boolean {
    return (
      node != null &&
      this.highlightedPtr.value != null &&
      (this.highlightedPtr.value.id == node.id || this.highlightedPtr.value.ck == (node as any).ck)
    );
  }

  /** Whether the node is currently selected. */
  isSelected(node: NodeKey<any> | null | undefined): boolean {
    return node != null && this.selectedPtrById.value[node.id!] != null;
  }

  /** Gets the current Selection */
  get selection(): SelectionData | null {
    return space.value?.selection ?? null;
  }

  /** Gets the inspection  */
  get inspection(): NodeReferenceData | null {
    return space.value?.inspectionPtr ?? null;
  }

  /** Gets the absolutely focused view components in bottom up order */
  get focusedViewComponents(): ViewComponent[] {
    return Object.values(this.focusedViewComponentsById.value);
  }

  /** The currently focused View */
  get focusedView(): ViewData | null {
    return this.focusedViewPtr.value != null ? this.graph.get(this.focusedViewPtr.value) : null;
  }

  /** Gets the node ptr of the given view */
  getViewNodePtr(
    view: ViewComponent,
    element?: HTMLElement | SVGElement | ViewComponent | null,
  ): NodeReferenceData | null {
    if (element != null) {
      const nodePtr = view?.exposed?.mapToNode?.(element);
      if (nodePtr != null) return nodePtr;
    }
    const nodePtr = this.graph.getMaybe(getViewComponentPtrMaybe(view))?.nodePtr ?? (view.props as ViewProps).nodePtr;
    return nodePtr ?? null;
  }

  /** Gets the view component for a certain view identity (self.id or anonymous id) */
  getViewComponent<T extends ViewComponent>(id: string): T | null {
    return this.viewRefsById.value[id] as T | null;
  }

  /** Resolve the view data */
  getViewData(view: SomeView): ViewData | null {
    if (view.metatype == ObjectType.VIEW) return view as ViewData;
    else return this.graph.get(view as NodeKey<NodeType.VIEW>) as ViewData | null;
  }

  findViewData(el: HTMLElement | ComponentInstance<any>): ViewData | null {
    const component = findViewComponentUp(el, isIdentifiedViewComponent);
    return component?.exposed.self?.value != null ? this.getViewData(component.exposed.self.value) : null;
  }

  /** Gets the node at the given element. */
  getNodeAt(el: HTMLElement | SVGElement): NodeReferenceData | null {
    // traverse upwards until we find some node ptr or a component that gives us a node ptr
    let nodePtr: NodeReferenceData | null = null;
    while (el != null) {
      if (el.dataset?.["suppressNode"] == "self") {
        el = el.parentElement!; // skip this element
        continue;
      } else if (el.dataset?.["nodeId"] != null) {
        // annotated element
        nodePtr = {
          metatype: ObjectType.NODE_REFERENCE,
          nodeType: Number(el.dataset["nodeType"]),
          id: el.dataset["nodeId"],
          ck: el.dataset["nodeCk"],
        };
        break;
      } else if ((el as any).__viewComponent != null) {
        // view component
        const component = (el as any).__viewComponent as ViewComponent;
        if (component.exposed.mapToNode != null) {
          nodePtr = component.exposed.mapToNode(el);
          if (nodePtr != null) break;
        } else if (component.props.nodePtr != null) {
          nodePtr = component.props.nodePtr;
          break;
        }
      }
      // up we go
      el = el.parentElement!;
    }

    // resolve against supergraph (to get full node ref)
    if (nodePtr != null) {
      const node = supergraph.get(nodePtr);
      if (node != null) nodePtr = toNodeRef(node);
    }

    return nodePtr;
  }

  /** Updates our internal focus state in response to a browser event */
  private onComponentFocused(element: ViewComponent | HTMLElement | SVGElement | null) {
    const component =
      element instanceof HTMLElement || element instanceof SVGElement ? findViewComponentUp(element) : element;

    // update component focus state
    if (component == null) {
      // reset
      this.focusedViewComponent.value = null;
      this.focusedViewComponentsById.value = {};
      this.focusedViewPtr.value = null;
    }

    // update focus
    this.focusedViewComponent.value = component;
    const componentsById: Record<string, ViewComponent> = {};
    const viewComponents = collectViewComponentsUp(component);
    viewComponents.forEach((c) => {
      componentsById[getViewComponentId(c)] = c;
    });
    this.focusedViewComponentsById.value = componentsById;
    this.focusedViewPtr.value = getViewComponentPtrMaybe(viewComponents.find(isIdentifiedViewComponent));
    const focusedView = this.graph.getMaybe(this.focusedViewPtr.value);

    // update root/inspection/base
    const rootViewComponentIdx = viewComponents.findIndex((v) => isViewComponentIn(v, ROOT_VIEW_TYPES));
    const baseView = this.graph.getMaybe(getViewComponentPtrMaybe(viewComponents[rootViewComponentIdx - 1]));
    const nodePtr = this.getNodeAt(element as HTMLElement);
    if (baseView != null && !HELPER_VIEW_TYPES.has(baseView.type) && nodePtr != null && !this.isInspected(nodePtr)) {
      this.inspect({ node: nodePtr, view: this.focusedViewPtr.value! });
    }
    if (this.focusedViewPtr.value != null && focusedView?.focusPtr?.id != nodePtr?.id) {
      this.focusInGraph({ view: this.focusedViewPtr.value, focusPtr: nodePtr });
    }
  }

  /** Selects this in the space. */
  select(selection: SelectionData | AnyNodeData[] | NodeReferenceData[] | undefined, options?: TransactionOptions) {
    if (selection != null && !isStruct(selection, StructType.SELECTION)) {
      selection = makeSelection(selection.map(toNodeRef));
    }
    if (space?.value != null && !deepValueEquals(space.value.selection, selection)) {
      this.tx().update(space.value, { selection }, { debounce: "long", ...options });
    }
  }

  /** Clears the selection */
  deselect() {
    if (this.selection != null) {
      this.select(undefined);
    }
  }

  /** Inspects the given node */
  inspect(inspect: {
    node: AnyNodeData | NodeReferenceData;
    view?: SomeView | ViewComponent | ComponentInstance<any> | HTMLElement | SVGElement | undefined;
  }): void {
    log.trace("canvas.inspect", inspect);

    // get node ptr
    const nodePtr = toNodeRef(inspect.node as AnyNodeData);
    let view: SomeView | null;
    if (isNodeRef(inspect.view) || isNode(inspect.view)) {
      view = inspect.view as SomeView;
    } else if (inspect.view != null) {
      view = this.findViewData(inspect.view);
    } else {
      view = this.focusedView;
    }
    if (view == null) {
      throw new Error(`no view for ${inspect.view}`);
    }
    if (inspectionPtr.value?.id != nodePtr.id) {
      const space = this.graph.getOrError(this.spacePtr.value!);
      this.tx().update(space, { inspectionPtr: nodePtr }, { debounce: "long" });
    }
    console.log("inspect", { nodePtr, view }); // nocheckin: set containerPtr/pagePtr

    // open inspector
    this.focusInGraph({ focusPtr: nodePtr, view: view });
  }

  /**
   * Focus the given view or node absolutely in the graph and in the component.
   * Also updates inspection to that node if re-focusing a view.
   * */
  focus(
    focus: (
      | { node: TypedNodeReferenceData<NodeType.VIEW> | ViewData }
      | { node: NodeReferenceData | AnyNodeData; view: SomeView }
    ) & { anchor?: FocusAnchor | NodeReferenceData; ignoreInspection?: boolean },
  ) {
    log.trace("canvas.focus", focus);
    focus.node = supergraph.getOrError({
      id: focus.node.id,
      ck: focus.node.id,
      nodeType: isNode(focus.node) ? (focus.node.metatype as unknown as NodeType) : focus.node.nodeType,
    }); // 'refresh' node in graph since it may have moved
    const nodeType = isNodeRef(focus.node) ? (focus.node as NodeReferenceData).nodeType : focus.node.metatype;

    if (nodeType == NodeType.VIEW && this.isInSpace(focus.node)) {
      // focus as a view in canvas
      const node = focus.node as ViewData | TypedNodeReferenceData<NodeType.VIEW>;

      // recover view & inspection from views' 'focus' down from focused view
      let viewData = this.getViewData(node);
      while (!HELPER_VIEW_TYPES.has(viewData?.type!) && viewData?.focusPtr?.id != null) {
        if (viewData?.focusPtr?.nodeType == NodeType.VIEW) {
          viewData = this.getViewData(viewData.focusPtr);
        } else if (!focus.ignoreInspection) {
          // auto-inspect what was previously focused inside this view
          this.inspect({ node: viewData!.focusPtr, view: viewData! });
          break;
        }
      }
      if (viewData == null) throw new Error(`no view data for ${describeNode(node)}`);

      // focus in graph & then in component
      this.focusInGraph({ ...focus, view: viewData });
      nextTick(() => {
        const focused = this.focusInComponent(node, focus.anchor);
        if (!focused) {
          const component = this.getViewComponent(viewData!.id!);
          if (component != null) this.onComponentFocused(component);
          log.trace("canvas.focusFailed", focus, { viewData, component });
        }
      });
    } else if ("view" in focus) {
      // focus as a general node in the given view
      const node = focus.node as NodeReferenceData | AnyNodeData;
      // focus in graph & then in component
      this.focusInGraph({ view: focus.view, focusPtr: toNodeRef(node) });
      if (!focus.ignoreInspection) {
        this.inspect({ node, view: focus.view });
      }
      const component = this.getViewComponent(focus.view.id!);
      if (component != null) this.focusInComponent(component, focus.anchor);
    } else {
      throw new Error(`unexpected focus: ${JSON.stringify(focus)}`);
    }
  }

  /** Focuses the given view absolutely in the graph. */
  focusInGraph(focus: { view: SomeView; focusPtr?: NodeReferenceData | null; parent?: SomeView; resetDown?: boolean }) {
    const tx = this.tx();

    // focus the given selection within the view
    if (focus.focusPtr != null) {
      const view = this.getViewData(focus.view)!;
      if (!deepValueEquals(view.focusPtr, focus.focusPtr)) {
        tx.update(view, { focusPtr: focus.focusPtr }, { debounce: "long" });
      }
    }

    // focus every 'child' in its 'parent' up to space root
    let child = this.getViewData(focus.view);
    if (child == null) throw new Error(`no view in graph for ${focus.view}`);
    let parent: ViewData | SpaceData | null = this.getViewData(focus.parent ?? child.parentPtr!);
    const updated = [];
    while (parent?.metatype == ObjectType.VIEW || parent?.metatype == ObjectType.SPACE) {
      const childFocus = toNodeRef(child);
      if (!deepValueEquals(parent.focusPtr, childFocus)) {
        tx.update(parent, { focusPtr: childFocus }, { debounce: "long" });
        updated.push(parent, { focusPtr: childFocus });
      }
      child = parent as ViewData;
      parent = this.graph.getMaybe(child.parentPtr) as ViewData | SpaceData | null;
    }

    // reset focus 'down' from view
    if (focus.resetDown) {
      const descendants = this.graph.getDescendants(child, { metatypes: [NodeType.VIEW] });
      descendants
        .filter((v) => v.focusPtr != null)
        .forEach((v) => tx.update(v, { focusPtr: undefined }, { debounce: "long" }));
    }
  }

  /** Focus the first focusable component within the given view. */
  focusInComponent(view: SomeView | ViewComponent, anchor?: FocusAnchor | NodeReferenceData): boolean {
    // get component/view data
    let component: ViewComponent | null;
    let viewData: ViewData | null;
    if ((view as SomeView).metatype != null) {
      component = this.getViewComponent((view as SomeView).id!);
      viewData = this.getViewData(view as SomeView);
    } else {
      component = view as ViewComponent;
      viewData = component.exposed?.self?.value != null ? this.getViewData(component.exposed.self.value) : null;
    }
    if (component == null) {
      // component currrently not available, log and bail
      const viewPtr = viewData ?? (view as ViewComponent).exposed?.self.value;
      const error = new Error(
        `no component for view: ${viewPtr != null ? describeNode(viewPtr) : getVueComponentType(view)}`,
      );
      log.trace("canvas.focusInComponent.error", { view, viewPtr, error });
      return false;
    }

    // if no anchor is given, try to use existing focus state
    if (anchor == null) {
      if (viewData?.focusPtr?.nodeType == NodeType.VIEW) {
        const child = this.getViewData(viewData.focusPtr);
        if (child != null) {
          // if we have a focus state we must use it, even if it didn't actually focus in the component
          //  (so we 'pretend' there was focus in the component by calling onComponentFocused directly)
          if (!this.focusInComponent(child)) this.onComponentFocused(component);
          return true;
        }
      } else {
        anchor = viewData?.focusPtr;
      }
    }

    // focus component directly or delegate
    if (component.exposed?.focus != null) {
      const focusResult = component.exposed?.focus(anchor ?? "top");
      if (typeof focusResult == "object") {
        if (focusResult instanceof HTMLElement) {
          focusResult.focus();
          this.onComponentFocused(focusResult); // immediately update active element
          return true;
        } else if (isViewComponent(focusResult)) {
          // an inner view component to focus
          if (this.focusInComponent(focusResult, anchor)) {
            return true;
          }
        }
      } else if (focusResult !== null && focusResult !== false) {
        // success (directly focused)
        return true;
      }
    }

    // fall back to focusing children
    if (viewData != null) {
      for (const child of this.graph.getChildren(viewData, NodeType.VIEW)) {
        if (this.focusInComponent(child)) return true;
      }
    }

    return false; // could not focus
  }

  /** Restores component focus to the currently absolutely focused element (if possible). */
  restoreComponentFocus(): boolean {
    if (this.space.value == null) throw new Error("no current space");
    if (this.space.value?.focusPtr != null) {
      const view = this.getViewData(this.space.value!.focusPtr!);
      if (view != null) {
        log.trace("canvas.restoreComponentFocus", view);
        return this.focusInComponent(view);
      }
    }
    return false;
  }

  /** Whether the given view is in absolute (top down) focus */
  isFocusedAbsolute(view: SomeView) {
    return this.focusedViewComponentsById.value[view.id!] != null;
  }

  /** Whether the given view is in absolute (top down) focus (reactive) */
  isFocusedAbsoluteRef(view: Ref<SomeView | undefined>): Ref<boolean> {
    return computed(() => view.value != null && this.isFocusedAbsolute(view.value));
  }

  /** Registers the current Vue component instance in the canvas with some View identity */
  registerView(self: Ref<NodeReferenceData | undefined>, id: Ref<string>) {
    const graph = this.graph;
    const tx = this.tx;
    const instance = getCurrentInstance() as ViewComponent | null;
    if (instance == null) throw new Error("no current Vue instance");

    //
    // mark element with component
    //

    function markEl() {
      // NOTE: we enforce that el must be a single element for all Views with a lint rule
      //  (unfortunately this doesn't prevent comments from forcing the root into a #text node during development, so we error below)
      const el = (instance as any).vnode.el as HTMLElement | null;
      if (!el) {
        log.warn("canvas.missingEl", getVueComponentType(instance), instance);
      } else if ((el as any).__viewComponent == null) {
        (el as any).__viewComponent = instance;
        if ("dataset" in el) el.dataset.view = "true";
        else throw new Error(`invalid root element in ${getVueComponentType(instance)}: ${el}`);
      }
    }
    onMounted(markEl);
    onUpdated(markEl);

    //
    // register component
    //

    function getComponentId(): string {
      // the :ViewComponentId is composed of the view id itself and any ancestor ids up to the next View
      // (so we can associate sub-View components that aren't full Views themselves with something consistent)
      if (self.value != null) {
        return self.value.id!;
      } else {
        const componentIdParts = [id.value];
        let ancestor = instance;
        while (ancestor != null && ancestor?.props?.self == null) {
          ancestor = (ancestor as any).parent;
          if (ancestor?.props.id != null) {
            componentIdParts.unshift(ancestor.props.id);
          }
        }
        return componentIdParts.join(".");
      }
    }

    // NOTE :Architecture: 'self'/'id' shouldn't change, so no need to watch in Canvas.registerView (?)
    // NOTE :Broken?: view componentId is incorrect inside pm-mounted view components (like a Flow in a Block on a Page)
    let componentId: string = getComponentId();
    watch(
      [self, id],
      () => {
        const viewRefsById = this.viewRefsById.value;
        if (componentId != null && viewRefsById[componentId] === instance) {
          delete viewRefsById[componentId];
        }

        // update registered component
        componentId = getComponentId();
        viewRefsById[componentId] = instance;
        triggerRef(this.viewRefsById);
      },
      { immediate: true },
    );
    // unregister
    onBeforeUnmount(() => {
      // should always be true, but maybe errored
      const viewRefsById = this.viewRefsById.value;
      if (componentId != null && viewRefsById[componentId] === instance) {
        delete viewRefsById[componentId];
        triggerRef(this.viewRefsById);
      }
    });

    //
    // Base View
    //

    // mark component with view (if we have one)
    if (self.value != null) {
      const viewRef = graph.getRef(self);
      (instance as any).__selfViewRef = viewRef;
    }

    let base = instance;
    while (base != null && (base as any)?.__selfViewRef == null) {
      base = (base as any).parent;
    }
    const baseViewRef: Ref<ViewData | null> = (base as any)?.__selfViewRef ?? ref(null);

    //
    // (Sub)State
    //

    function getState(viewId?: string, override?: ViewProps): Partial<Record<string, any> | undefined> {
      const subviewPacked = (baseViewRef?.value?.subviewsPacked as any)?.[viewId ?? componentId];
      if (subviewPacked == null) return undefined;
      const view = unpackBuiltinObject(subviewPacked, ObjectType.VIEW);
      if (override != null) {
        Object.assign(view, override);
      }
      return view;
    }

    function getChildState(viewId: string, override?: ViewProps): Partial<Record<string, any> | undefined> {
      return getState(componentId + "." + viewId, override);
    }

    function update(update: Partial<NodeIn<NodeType.VIEW>>, options?: TransactionOptions) {
      const baseView = baseViewRef?.value;
      if (baseView == null) return; // no base view, cannot update (should error?)
      if (self.value != null) {
        // base view upate
        tx()
          .with({ category: ChangeCategory.SPACE })
          .update(baseView, update as NodeIn<any>, options);
      } else {
        // subview update
        if ("subnode" in update && "type" in update) {
          // merge current subnode into new subnode
          // (we override subview values at the property level, so this would get lost otherwise)
          const key = update.type!.toString();
          if ((instance?.props?.subnodePacked as any)?.[key] != null) {
            const subnode = unpackSubnode(NodeType.VIEW, update.type!, instance!.props.subnodePacked as any);
            update.subnode = { ...subnode, ...update.subnode };
          }
        }
        const edits = makeEdit(baseView, update as NodeIn<any>);
        for (const edit of edits) {
          edit.path = [SUBVIEWS_PROPERTY_KEY, componentId, ...edit.path];
        }
        tx().with({ category: ChangeCategory.SPACE }).update(baseView, edits, options);
      }
    }

    //
    // Selection
    // (just forward to/from space)
    //

    function select(
      selection: SelectionData | AnyNodeData[] | NodeReferenceData[] | undefined,
      options?: TransactionOptions,
    ) {
      canvas.select(selection, options);
    }

    function deselect(options?: TransactionOptions) {
      select(undefined, options);
    }

    function isSelected(node: NodeKey<any>): boolean {
      return canvas.isSelected(node);
    }

    return { getState, getChildState, update, select: select, deselect: deselect, isSelected, baseViewRef };
  }

  /** Gets the containing root view (or self, if any) for a view */
  getRootView(view: ViewData): ViewData {
    if (ROOT_VIEW_TYPES.has(view.type)) return view;
    const ancestors = this.graph.getAncestors(view, { metatypes: [NodeType.VIEW], includeSelf: true });
    return ancestors.find((v) => ROOT_VIEW_TYPES.has(v.type)) ?? view;
  }

  /** Whether a given node is a view in our space */
  isInSpace(node: NodeKey<any>): boolean {
    return isDescendantOf(this.graph, node, this.spacePtr.value!);
  }

  /** Gets all the frames non-reactively (direct children of Window/Split views) */
  get frames(): ViewData[] {
    if (this.spacePtr.value == null) return [];
    const getFrames = (view: ViewData): ViewData[] => {
      if (view.type == ViewType.WINDOW || view.type == ViewType.SPLIT) {
        return this.graph.getChildren(view, NodeType.VIEW).flatMap(getFrames);
      } else {
        return [view];
      }
    };
    const frames = this.graph
      .getChildren(this.spacePtr.value, NodeType.VIEW)
      .flatMap(getFrames)
      .filter((v) => v.type != ViewType.SPLIT);
    return frames;
  }

  /** Gets all the views non-reactively */
  get views(): ViewData[] {
    if (this.spacePtr.value == null) return [];
    return this.graph.getDescendants(this.spacePtr.value, { metatypes: [NodeType.VIEW] });
  }

  get activeViews(): ViewData[] {
    if (this.spacePtr.value == null) return [];
    return this.getActiveViewsIn(this.spacePtr.value);
  }

  /** Gets all the *active* Views descendant from the given Node (ignoring inactive History views) */
  getActiveViewsIn(nodePtr: NodeReferenceData): ViewData[] {
    // this.graph.getDescendants(this.spacePtr.value, { metatypes: [NodeType.VIEW] })
    const graph = this.graph;
    const descendants: ViewData[] = [];

    // walk children but ignore inactive views
    function walkChildren(node: AnyNodeData) {
      if (!isNode(node, NodeType.VIEW)) {
        // skip non-Views
        for (const child of graph.getChildren(node, NodeType.VIEW)) {
          walkChildren(child);
        }
        return;
      }

      descendants.push(node);
      const children = graph.getChildren(node, NodeType.VIEW);
      if (node.type == ViewType.HISTORY) {
        const focusedId = node.focusPtr?.id;
        if (focusedId != null) {
          const focusedView = children.find((v) => v.id == focusedId);
          if (focusedView != null) {
            walkChildren(focusedView);
          }
        }
      } else {
        for (const child of children) {
          walkChildren(child);
        }
      }
    }

    // start at root
    const root = graph.get(nodePtr);
    if (root == null) throw new Error(`no root for: ${describeNode(nodePtr)}`);
    walkChildren(root);
    return descendants;
  }

  /** Finds a view with properties exactly like the criteria */
  findView(like: {
    type: ViewType;
    nodePtr?: NodeReferenceData | undefined;
    filter?: (view: ViewData) => boolean;
    isActive?: boolean;
  }): ViewData | null {
    if (Object.keys(like).length == 0) return null;
    if (this.spacePtr.value == null) return null;
    const views = like.isActive ? this.getActiveViewsIn(this.spacePtr.value) : this.views;
    const match = views.find((v) => {
      if (v.type != like.type) return false;
      if (like.nodePtr != null && v.nodePtr?.id != like.nodePtr?.id) return false;
      if (like.filter != null && !like.filter(v)) return false;
      return true;
    });
    return match ?? null;
  }

  /** Add a new view to the canvas at the current root.  */
  addView(view: ViewIn, options?: OpenViewOptions) {
    let tx = this.tx();
    // NOTE :UX: should we always ignore inactive views? would it be useful to consider them sometimes?
    //  (like when there's an inactive view like the one you want in history, maybe we should duplicate that)
    const existing = this.findView({ type: view.type, nodePtr: view.nodePtr, filter: options?.filter, isActive: true });

    if (existing == null || options?.ifPresent == null || options?.ifPresent == "duplicate") {
      // find root
      let parent =
        this.views.find((v) => v.type == ViewType.TAB || v.type == ViewType.HISTORY) ??
        this.views.find((v) => v.type == ViewType.WINDOW || v.type == ViewType.SPLIT);
      if (parent == null) {
        // empty space, create default
        const { primary } = createDesktopEmptySpace(tx, this.space.value!);
        parent = primary;
      }

      log.trace("canvas.addView.create", view, { existing, options, parent });
      let siblings = this.graph.getChildren(parent, NodeType.VIEW);
      tx = tx.with({ change: { key: newChangeId() } });

      // prune history from current focused tab
      if (isNode(parent, NodeType.VIEW) && parent.type == ViewType.HISTORY) {
        const focusedId = parent.focusPtr?.id;
        const focusedTabIdx = siblings.findIndex((tab) => tab.id == focusedId) ?? -1;
        for (let i = focusedTabIdx + 1; i < siblings.length; i++) {
          tx.delete(siblings[i]);
        }
        siblings = siblings.slice(0, focusedTabIdx + 1);
      }

      // create & focus
      const newView = makeNode({
        ...view,
        metatype: NodeType.VIEW,
        packagePtr: parent.packagePtr,
        orderKey: generateOrderKey(siblings[-1]?.orderKey ?? null, null),
        parentPtr: toNodeRef(parent),
        icon: toIconMaybe(view.icon),
        mode: view.mode ?? NodeMode.MAIN,
      });
      if ((view.name ?? "").length == 0) {
        newView.name = generateNodeName(
          newView,
          this.graph.getDescendants(this.spacePtr.value!, { metatypes: [NodeType.VIEW] }),
        );
      }
      tx.create(newView);
      this.focus({ node: newView });
      return newView;
    } else if (options?.ifPresent == "focus") {
      log.trace("canvas.addView.focus", view, { existing, options });
      this.focus({ node: existing });
      return existing;
    } else if (options?.ifPresent == "upsertAndFocus") {
      log.trace("canvas.addView.upsertAndFocus", view, { existing, options });
      if (view.subnode != null && view.subnodePacked == null) {
        view.subnodePacked = packSubnode(NodeType.VIEW, view.type, view.subnode);
      }
      for (const property of [
        ViewProperty.title,
        ViewProperty.icon,
        ViewProperty.focusPtr,
        ViewProperty.subnodePacked,
      ]) {
        const propertyName = ViewProperty[property];
        if (!deepValueEquals((existing as any)[propertyName], (view as any)[propertyName])) {
          tx.update(existing, { [propertyName]: (view as any)[propertyName] }, { debounce: "tick" });
        }
      }
      this.focus({ node: existing });
      return existing;
    } else {
      assertNever(options?.ifPresent);
    }
  }

  /**
   * Goes to the given node, whatever that means. Unlike addView, this upserts the view by default.
   * If it's a view node, we focus it in the space graph (it must exist).
   * For any other node, we find or open an appropriate view for it and focus that somehow.
   */
  goToNode(
    node: AnyNodeData | NodeReferenceData | null,
    options?: { graph?: ReadNodeGraph; skipSelf?: boolean } & OpenViewOptions,
  ): void {
    let nodePtr = isNodeRef(node) ? node : toNodeRef(node as AnyNodeData);
    const graph = options?.graph ?? this.graph;
    log.debug("canvas.goToNode", node);
    // reload from graph
    node = supergraph.get(nodePtr) ?? node;
    if (node == null) {
      // not found
      throw new Error(`node not found in ${describeScope(graph.scope)}: ${describeNode(nodePtr)}`);
    }

    // focus views directly
    if (nodePtr.nodeType == NodeType.VIEW && this.isInSpace(node)) {
      this.focus({ node: nodePtr as ViewData | TypedNodeReferenceData<NodeType.VIEW> });
      return;
    }

    // open flow
    if (
      isNode(node, NodeType.FLOW) ||
      (isNode(node, NodeType.ACTION) && node.parentPtr?.nodeType == NodeType.FLOW) ||
      isNode(node, NodeType.TRANSITION) ||
      (isNode(node, NodeType.FIELD) && getContainingFlow(graph, node) != null)
    ) {
      const containingFlow = getContainingFlow(graph, node);
      if (!containingFlow) throw new Error(`no containing flow for: ${describeNode(node)}`);
      const view = this.addView(
        {
          type: ViewType.FLOW,
          nodePtr: toNodeRef(containingFlow),
          focusPtr: toNodeRef(node),
          ...options?.props,
        },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: nodePtr, view });
      return;
    }

    // open implementation
    if (isNode(node, NodeType.KIT) || (isNode(node, NodeType.ACTION) && node.parentPtr?.nodeType == NodeType.KIT)) {
      if (isNode(node, NodeType.ACTION)) {
        node = supergraph.getOrError(node.parentPtr!);
        nodePtr = toNodeRef(node);
      }
      const view = this.addView(
        { type: ViewType.KIT, nodePtr: toNodeRef(node), ...options?.props },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: nodePtr, view });
      return;
    }

    // open database
    if (
      (isNode(node, NodeType.BLOCK) && node.type == BlockType.DATABASE) ||
      isNode(node, NodeType.DATABASE) ||
      isNode(node, NodeType.RECORD)
    ) {
      let view: ViewData;
      if (isNode(node, NodeType.RECORD)) {
        const database = graph.get(node.databasePtr!);
        if (database == null) throw new Error(`no containing database for record: ${describeNode(node)}`);
        view = this.addView(
          {
            type: ViewType.DATABASE,
            nodePtr: toNodeRef(database),
            focusPtr: toNodeRef(node),
            ...options?.props,
          },
          { ifPresent: "upsertAndFocus", ...options },
        );
      } else {
        if (isNode(node, NodeType.BLOCK)) {
          // unwrap DatabaseBlock to Database
          node = supergraph.getOrError(node.nodePtr!);
        }
        view = this.addView(
          { type: ViewType.DATABASE, nodePtr: toNodeRef(node), ...options?.props },
          { ifPresent: "upsertAndFocus", ...options },
        );
      }
      this.inspect({ node: nodePtr, view });
      return;
    }

    // open chat
    if (isNode(node, NodeType.MESSAGE) || isNode(node, NodeType.THREAD) || isNode(node, NodeType.CHANNEL)) {
      let inspectPtr: NodeReferenceData | undefined;
      let threadPtr: NodeReferenceData | undefined;
      let channelPtr: NodeReferenceData;
      if (isNode(node, NodeType.MESSAGE)) {
        inspectPtr = nodePtr;
        threadPtr = node.threadPtr;
        channelPtr = node.channelPtr!;
      } else if (isNode(node, NodeType.THREAD)) {
        inspectPtr = nodePtr;
        threadPtr = nodePtr;
        channelPtr = node.channelPtr!;
      } else if (isNode(node, NodeType.CHANNEL)) {
        inspectPtr = nodePtr;
        threadPtr = undefined;
        channelPtr = nodePtr;
      } else {
        assertNever(node);
      }
      const view = this.addView(
        { type: ViewType.THREAD, nodePtr: threadPtr ?? channelPtr },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: inspectPtr, view });
      return;
    }

    // open page
    if (
      isNode(node, NodeType.PAGE) ||
      isNode(node, NodeType.BLOCK) ||
      (isInlineNode(node) && node.definitionPtr != null)
    ) {
      const containingPage = graph
        .getAncestors(nodePtr, { metatypes: [NodeType.PAGE], includeSelf: !options?.skipSelf })
        .find((p) => true);
      if (containingPage) {
        const view = this.addView(
          { type: ViewType.PAGE, nodePtr: toNodeRef(containingPage), focusPtr: toNodeRef(nodePtr), ...options?.props },
          { ifPresent: "upsertAndFocus", ...options },
        );
        this.inspect({ node: nodePtr, view });
        return;
      }
    }

    // focus on runnable source + run
    // (set as Space.run_ptr and open containing Run view in Help)
    if (isNode(node, NodeType.RUN) || isNode(node, NodeType.INTERRUPTION)) {
      const basePtr = getBaseFromNode(node);
      const base = basePtr != null ? graph.get(basePtr) : null;
      if (base != null) {
        this.goToNode(base, options);
        const runPtr = isNode(node, NodeType.RUN) ? (node.rootPtr ?? toNodeRef(node)) : node.rootPtr;
        this.tx().update(this.space.value!, { runPtr }, { debounce: "tick" });
        const helpView = this.findView({ type: ViewType.CONTEXT });
        if (helpView != null) {
          this.tx().update(
            helpView,
            makeEditFromSubnode(helpView, { metatype: NodeType.VIEW, type: ViewType.CONTEXT }),
          );
        }
        return;
      }
    }

    // open computer
    if (isNode(node, NodeType.COMPUTER)) {
      const view = this.addView(
        { type: ViewType.COMPUTER, nodePtr: toNodeRef(node) },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: nodePtr, view });
      return;
    }

    // can't go there
    else {
      toaster.error({
        icon: getNodeIcon(node),
        title: `Can't Open ${getNodeTitle(node as AnyNodeData) ?? "This Node"}`,
        text: `There is no view for ${toCamelName(ObjectType, node.metatype)}s`,
      });
    }
  }

  /**
   * Removes the given view from the space graph, taking care to clean up.
   */
  removeView(graph: ReadNodeGraph, view: ViewData) {
    log.trace("canvas.remove", view);
    const parent = graph.get(view.parentPtr!) as ViewData;
    this.tx().delete(view);
    this.cleanupRootViews(graph, parent);
  }

  /**
   * Cleanup previously split (sub-)root views that are no longer needed.
   * NOTE: right now we only close sub root views because it's annoying to have your layout change because you accidentally close a tab.
   *  (and the 'layout' is usually your root splits)
   */
  cleanupRootViews(graph: ReadNodeGraph, view: ViewData) {
    if (!ROOT_VIEW_TYPES.has(view.type)) return;
    if (
      graph.getChildren(view, NodeType.VIEW).length == 0 &&
      graph.getChildren(view.parentPtr!, NodeType.VIEW).length > 1 &&
      graph.get({ nodeType: NodeType.VIEW, id: view.parentPtr!.id })?.type == ViewType.SPLIT
    ) {
      // NOTE :UX: should we re-distribute space if cleaning up after a split?
      this.removeView(graph, view);
    }
  }

  /**
   * Adds the given view into this view at the target/anchor.
   */
  moveView(
    graph: ReadNodeGraph,
    move: { self: ViewData; child: ViewData; anchor: "start" | "end"; referenceId?: string | null },
    options?: { tx?: Transaction },
  ) {
    log.trace("canvas.move", { graph, ...move });
    const { self, child, anchor, referenceId } = move;
    const tx = options?.tx ?? this.tx();
    // move & update order
    if (child.id != referenceId) {
      updateOrder({
        tx,
        node: child,
        position: anchor == "start" ? "before" : "after",
        reference: referenceId ?? null,
        getNodes: () => graph.getChildren(self, NodeType.VIEW),
      });
    }
    if (child.parentPtr?.id != self.id) {
      tx.move(child, { parentPtr: toNodeRef(self) }, { debounce: "tick" });
      this.cleanupRootViews(graph, graph.get(child.parentPtr!) as ViewData);
    }
  }

  /**
   * 'Splits' the 'parent' view to accomodate a new equally sized subview 'child' (at the anchor).
   * If we're already split alongside the given orientation, the child is added to the existing split.
   */
  splitView(
    graph: ReadNodeGraph,
    split: { parent: ViewData; child: ViewData; anchor: Omit<SplitAnchor, "center">; duplicateIfSelf?: boolean },
  ) {
    log.trace("canvas.split", { graph, ...split });
    // eslint-disable-next-line prefer-const
    let { parent, child, anchor } = split;
    const tx = this.tx();

    // determine if we need a new split in the enclosing split view
    let splitView: ViewData | null = null;
    if (parent.type == ViewType.WINDOW) splitView = parent;
    else if (parent.parentPtr != null) splitView = graph.get(parent.parentPtr) as ViewData;
    if (splitView?.metatype != ObjectType.VIEW)
      throw new Error(
        `no enclosing split view: [parent=${ObjectType[parent.metatype]}, parent.type=${ViewType[parent.type]}]`,
      );
    const isHorizontal = anchor == "left" || anchor == "right";
    const orientation = isHorizontal ? Orientation.HORIZONTAL : Orientation.VERTICAL;
    const isOrderFlipped = anchor == "right" || anchor == "bottom";
    const needsNewSplit = (splitView.orientation ?? DEFAULT_ORIENTATION) != orientation;

    // duplicate child if it belongs to self
    if (child.parentPtr?.id == parent.id && split.duplicateIfSelf) {
      child = cloneNode(tx, graph, child, { includeChildren: true });
      tx.create(child);
    }

    if (needsNewSplit) {
      // insert a new split in place of 'self'
      const split = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.WINDOW,
        parentPtr: parent.parentPtr,
        packagePtr: parent.packagePtr,
        orderKey: parent.orderKey,
        size: parent.size,
        name: parent.name,
        orientation,
      });
      tx.create(split);
      tx.move(parent, {
        parentPtr: toNodeRef(split),
        size: undefined,
        orderKey: isOrderFlipped ? "a0" : "a1",
      });

      // and a new tab wrapper
      const childWrapper = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
        parentPtr: toNodeRef(split),
        packagePtr: parent.packagePtr,
        name: `${parent.name}${toCasing(anchor.toUpperCase(), Casing.CAMEL)}`,
        orderKey: isOrderFlipped ? "a1" : "a0",
      });
      tx.create(childWrapper);
      tx.move(child, {
        parentPtr: toNodeRef(childWrapper),
        size: undefined,
        orderKey: "a0",
      });
    } else {
      // 'split' size between self and child with a new tab wrapper
      const halfSize = splitBox(parent.size!);
      const newSplitParent = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
        parentPtr: parent.parentPtr,
        packagePtr: parent.packagePtr,
        name: `${parent.name}Split`,
        size: halfSize,
        orderKey: getOrderKey({
          nodes: graph.getChildren(splitView, NodeType.VIEW),
          position: isOrderFlipped ? "after" : "before",
          reference: parent,
        }),
      });
      tx.create(newSplitParent);
      tx.move(child, { parentPtr: toNodeRef(newSplitParent), size: undefined, orderKey: "a0" });
      tx.update(parent, { size: halfSize });
    }
    this.cleanupRootViews(graph, graph.get(child.parentPtr!) as ViewData);
  }

  /** Opens a new popover */
  pushPopover(
    push: {
      generation?: number | "new";
      trigger: HTMLElement | SVGElement;
      reference: { x: number; y: number } | HTMLElement | SVGElement;
    } & PopoverInfoIn,
  ): PopoverInstance {
    // simple wrapper for now (should probably move Popover state into SpaceCanvas)
    return pushPopover(push);
  }
}

function makeMainWindow(space: SpaceData, tx: Transaction): ViewData {
  const main = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.WINDOW,
    parentPtr: toNodeRef(space),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Main",
  });
  tx.create(main);
  return main;
}

/** Clears all views from canvas */
export function clearSpace(tx: Transaction, graph: ReadNodeGraph, space: SpaceData) {
  const roots = graph.getChildren(space, NodeType.VIEW);
  for (const root of roots) {
    tx.delete(root);
  }
  tx.update(
    space,
    { focusPtr: undefined, inspectionPtr: undefined, containerPtr: undefined, pagePtr: undefined },
    { debounce: "long" },
  );
}

type ViewLayoutIn = {
  children?: ViewLayoutIn[];
} & ViewIn;

/** Recursively create the views for a layout */
function makeLayout(
  tx: Transaction,
  root: SpaceData | ViewData,
  layout: ViewLayoutIn[],
): {
  viewsByName: Record<string, ViewData>;
} {
  const viewsByName: Record<string, ViewData> = {};
  const viewsByType: Partial<Record<ViewType, ViewData[]>> = {};

  function doMakeLayoutRec(parent: SpaceData | ViewData, viewIn: ViewLayoutIn, ancestors: ViewData[]) {
    const name =
      viewIn.name ?? generateNodeName({ ...viewIn, metatype: ObjectType.VIEW }, viewsByType[viewIn.type] ?? []);
    const view = tx.create({
      metatype: NodeType.VIEW,
      title: viewIn.name ?? toCamelName(ViewType, viewIn.type),
      ...viewIn,
      name,
      parentPtr: toNodeRef(parent),
      packagePtr: root.packagePtr,
    });
    if (viewsByName[view.name!] != null) throw new Error(`duplicate view name: ${view.name}`);
    viewsByName[view.name!] = view;
    if (viewsByType[view.type] == null) viewsByType[view.type] = [];
    viewsByType[view.type]!.push(view);

    // descend
    if (viewIn.children != null && viewIn.children.length > 0) {
      const orderKeys = generateOrderKeys(null, null, viewIn.children.length);
      for (let i = 0; i < viewIn.children.length; i++) {
        const child = viewIn.children[i];
        doMakeLayoutRec(view, { ...child, orderKey: orderKeys[i] }, [...ancestors, view]);
      }
    }
  }

  const orderKeys = generateOrderKeys(null, null, layout.length);
  for (let i = 0; i < layout.length; i++) {
    const child = layout[i];
    doMakeLayoutRec(root, { ...child, orderKey: orderKeys[i] }, []);
  }

  return { viewsByName };
}

/** Creates the default three-side canvas */
export function createDesktopDefaultSpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  const window = makeMainWindow(space, tx);
  const layout = makeLayout(tx, window, [
    {
      type: ViewType.SIDEBAR,
      name: "Sidebar",
      size: makeStruct({ metatype: StructType.RECTANGLE, width: 320 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 300, maxWidth: 600 }),
    },
    {
      type: ViewType.HISTORY,
      name: "Main",
      size: makeStruct({ metatype: StructType.RECTANGLE, widthRelative: 1000 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 600 }),
    },
    {
      type: ViewType.CONTEXT,
      name: "Alternate",
      orientation: Orientation.VERTICAL,
      size: makeStruct({ metatype: StructType.RECTANGLE, width: 600 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 600 }),
    },
  ]);
  return { primary: layout.viewsByName["Main"] };
}

/** Creates an empty desktop space */
export function createDesktopEmptySpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  const window = makeMainWindow(space, tx);
  const layout = makeLayout(tx, window, [
    {
      type: ViewType.HISTORY,
      name: "Main",
      size: makeStruct({ metatype: StructType.RECTANGLE, widthRelative: 1000 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 600 }),
    },
  ]);
  return { primary: layout.viewsByName["Main"] };
}

//
// Actions
//

/** Duplicates the selected nodes */
export function duplicateSelection(command: Command, ctx: CommandContext): boolean {
  const { connection, graph, nodes } = getNodesForCommand(command, ctx);
  if (connection == null || graph == null || nodes.length == 0) {
    return true; // no action, but suppress anyway to avoid triggering browser shortcuts
  }
  const clonedNodes = cloneNodes(
    connection.tx,
    graph,
    nodes.filter((node) => !NON_DELETABLE_NODE_TYPES.includes(node.metatype as unknown as NodeType)),
  );
  canvas.select(clonedNodes);
  if (clonedNodes.length > 0 && !(ctx.event != null && findViewComponentUp(ctx.event.target, HELPER_VIEW_TYPES))) {
    canvas.goToNode(clonedNodes[0]);
    canvas.inspect({ node: clonedNodes[0], view: canvas.focusedViewPtr.value });
  }
  return true;
}

/** Deletes the selected nodes */
export function deleteSelection(command: Command, ctx: CommandContext): boolean {
  const { connection, graph, nodes } = getNodesForCommand(command, ctx);
  if (connection == null || graph == null || nodes.length == 0) {
    return false; // bubble up
  }
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Delete" } });
  for (const node of nodes.filter((node) => !NON_DELETABLE_NODE_TYPES.includes(node.metatype as unknown as NodeType))) {
    tx.delete(node);
  }
  return true;
}

/** Archives the selected nodes */
export function archiveSelection(command: Command, ctx: CommandContext): boolean {
  const { connection, graph, nodes } = getNodesForCommand(command, ctx);
  if (connection == null || graph == null || nodes.length == 0) {
    return false; // bubble up
  }
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Archive" } });
  for (const node of nodes) {
    tx.archive(node);
  }
  return true;
}

// space
declareCommands<"space">({
  "space.omnibar.bench": {
    title: "Search everything",
    text: "Search everything",
    shortcuts: ["mod+k"],
  },
  "space.omnibar.commands": {
    title: "Search commands",
    text: "Search for a command",
    shortcuts: ["ctrl+k"],
  },
  // edit
  "space.edit.rename": {
    icon: "fas fa-pencil",
    title: "Rename",
    text: "Rename this item",
    shortcuts: ["f2"],
  },
  "space.edit.move": {
    icon: "fas fa-arrows-turn-right",
    title: "Move",
    text: "Move this item",
  },
  "space.edit.copy": {
    icon: "fas fa-copy",
    title: "Copy",
    text: "Copy this item",
    shortcuts: ["mod+c"],
  },
  "space.edit.cut": {
    icon: "fas fa-scissors",
    title: "Cut",
    text: "Cut this item",
    shortcuts: ["mod+x"],
  },
  "space.edit.paste": {
    icon: "fas fa-paste",
    title: "Paste",
    text: "Paste this item",
    shortcuts: ["mod+v"],
  },
  "space.edit.duplicate": {
    icon: "fas fa-clone",
    title: "Duplicate",
    text: "Duplicate this item",
    shortcuts: ["mod+d"],
    command: (command, ctx) => duplicateSelection(command, ctx),
  },
  "space.edit.archive": {
    icon: "fas fa-archive",
    title: "Archive",
    text: "Archive this item",
    shortcuts: ["mod+shift+a"],
    command: (command, ctx) => archiveSelection(command, ctx),
  },
  "space.edit.delete": {
    icon: "fas fa-trash",
    title: "Delete",
    text: "Delete this item",
    shortcuts: ["del", "backspace"],
    command: (command, ctx) => deleteSelection(command, ctx),
  },
  // navigate
  "space.navigate.open": {
    icon: "fas fa-arrow-up-right",
    title: "Open",
    text: "Open this node in a new view",
    shortcuts: ["mod+enter"],
    command: (command, ctx) => {
      const { connection, graph, nodes } = getNodesForCommand(command, ctx);
      if (connection == null || graph == null || nodes.length == 0) {
        return false; // bubble up
      }
      canvas.goToNode(nodes[0]);
      return true;
    },
  },
  "space.navigate.up": {
    icon: "fas fa-arrow-up",
    title: "Navigate Up",
    text: "Navigate up",
    shortcuts: ["up"],
  },
  "space.navigate.down": {
    icon: "fas fa-arrow-down",
    title: "Navigate Down",
    text: "Navigate down",
    shortcuts: ["down"],
  },
  "space.navigate.left": {
    icon: "fas fa-arrow-left",
    title: "Navigate Left",
    text: "Navigate left",
    shortcuts: ["left"],
  },
  "space.navigate.right": {
    icon: "fas fa-arrow-right",
    title: "Navigate Right",
    text: "Navigate right",
    shortcuts: ["right"],
  },
  "space.navigate.zoomIn": {
    icon: "fas fa-search-plus",
    title: "Zoom In",
    text: "Zoom in",
    shortcuts: ["plus", "mod+plus"],
  },
  "space.navigate.zoomOut": {
    icon: "fas fa-search-minus",
    title: "Zoom Out",
    text: "Zoom out",
    shortcuts: ["minus", "mod+minus"],
  },
  "space.navigate.reset": {
    icon: "fas fa-arrows-to-dot",
    title: "Reset",
    text: "Reset",
    shortcuts: ["0"],
  },
  "space.navigate.enter": {
    icon: "fas fa-arrow-in",
    title: "Navigate In",
    text: "Navigate in",
    shortcuts: ["enter"],
  },
  "space.navigate.exit": {
    icon: "fas fa-arrow-out",
    title: "Navigate Out",
    text: "Navigate out",
    shortcuts: ["esc"],
  },
  "space.navigate.pageUp": {
    icon: "fas fa-arrow-up-to-line",
    title: "Page Up",
    text: "Page up",
    shortcuts: ["pageup"],
  },
  "space.navigate.pageDown": {
    icon: "fas fa-arrow-down-to-line",
    title: "Page Down",
    text: "Page down",
    shortcuts: ["pagedown"],
  },
  "space.navigate.goBackward": {
    icon: "fas fa-arrow-turn-left",
    title: "Go Back",
    text: "Go back in view history",
    shortcuts: ["mod+shift+backspace"],
  },
  "space.navigate.goForward": {
    icon: "fas fa-arrow-turn-right",
    title: "Go Forward",
    text: "Go forward in view history",
    shortcuts: ["mod+shift+enter"],
  },
  // select
  "space.select.all": {
    icon: "fas fa-check-square",
    title: "Select All",
    text: "Select all items",
    shortcuts: ["mod+a"],
  },
  "space.select.up": {
    icon: "fas fa-square-caret-up",
    title: "Select Up",
    text: "Select up",
    shortcuts: ["shift+up"],
  },
  "space.select.down": {
    icon: "fas fa-square-caret-down",
    title: "Select Down",
    text: "Select down",
    shortcuts: ["shift+down"],
  },
  "space.select.left": {
    icon: "fas fa-square-caret-left",
    title: "Select Left",
    text: "Select left",
    shortcuts: ["shift+left"],
  },
  "space.select.right": {
    icon: "fas fa-square-caret-right",
    title: "Select Right",
    text: "Select right",
    shortcuts: ["shift+right"],
  },
  "space.select.clear": {
    icon: "fas fa-times",
    title: "Clear Selection",
    text: "Clear selection",
    shortcuts: ["esc"],
    command: (command, ctx) => {
      if (canvas.selection != null) {
        canvas.deselect();
      }
    },
  },
  // move
  "space.move.up": {
    icon: "fas fa-square-up",
    title: "Move Up",
    text: "Move up",
    shortcuts: ["alt+up"],
  },
  "space.move.down": {
    icon: "fas fa-square-down",
    title: "Move Down",
    text: "Move down",
    shortcuts: ["alt+down"],
  },
  "space.move.left": {
    icon: "fas fa-square-left",
    title: "Move Left",
    text: "Move left",
    shortcuts: ["alt+left", "shift+tab"],
  },
  "space.move.right": {
    icon: "fas fa-square-right",
    title: "Move Right",
    text: "Move right",
    shortcuts: ["alt+right", "tab"],
  },
  // search
  "space.search.findInView": {
    icon: "fas fa-magnifying-glass",
    title: "Search in View",
    text: "Find in this view",
    shortcuts: ["mod+f"],
  },
  "space.search.replaceInView": {
    icon: "fas fa-right-left",
    title: "Replace in View",
    text: "Replace in this view",
    shortcuts: ["mod+r"],
  },
  "space.search.findInSpace": {
    icon: "fas fa-magnifying-glass",
    title: "Search in Space",
    text: "Find in this space",
    shortcuts: ["mod+shift+f"],
  },
  "space.search.replaceInSpace": {
    icon: "fas fa-right-left",
    title: "Replace in Space",
    text: "Replace in this space",
    shortcuts: ["mod+shift+r"],
  },
  // launch
  "space.launch.discord": {
    title: "Open Discord",
    text: "Join the community on Discord",
    icon: "fab fa-discord",
    url: DISCORD_URL,
    command: () => {
      // open in new tab
    },
  },
  "space.launch.fullscreen": {
    type: "toggle",
    icon: "fas fa-maximize",
    title: "Toggle Fullscreen",
    text: "Toggle fullscreen mode",
    command: () => {
      const isFullscreen = document.fullscreenElement != null;
      if (isFullscreen) {
        document.exitFullscreen();
      } else {
        document.documentElement.requestFullscreen();
      }
    },
  },
  // create
  "space.create.page": {
    title: "Create Page",
    icon: "far fa-file",
    text: "Create a new page",
    shortcuts: ["ctrl+n"],
    command: () => {
      const page = createPage(benchConnection.tx, benchGraph, { anchor: "inside", target: pkg.value!, page: {} });
      canvas.goToNode(page);
    },
  },
  "space.create.thread": {
    title: "Create Thread",
    icon: "fas fa-reel",
    text: "Create a new thread",
    shortcuts: ["ctrl+t"],
    command: () => {
      const thread = createThread(benchConnection.tx, benchGraph, {
        thread: {
          parentPtr: toNodeRef(pkg.value!),
          packagePtr: toNodeRef(pkg.value!),
        },
        members: [BENCH_BENCH_AGENT_PTR, "user"],
      });
      canvas.goToNode(thread);
    },
  },
});

// view
declareCommands<"view">({
  // history
  "view.history.goBackward": {
    icon: "fas fa-chevron-left",
    title: "Go Back",
    text: "Go back to the previous view",
    shortcuts: ["mod+shift+backspace"],
  },
  "view.history.goForward": {
    icon: "fas fa-chevron-right",
    title: "Go Forward",
    text: "Go forward to the next view",
    shortcuts: ["mod+shift+enter"],
  },
  // navigate
  "view.navigate.duplicateTab": {
    icon: "fas fa-copy",
    title: "Duplicate Tab",
    text: "Duplicate the current tab",
  },
  "view.navigate.focusPreviousTab": {
    icon: "fas fa-chevron-left",
    title: "Focus Previous Tab",
    text: "Navigate to the previous tab",
    shortcuts: ["alt+shift+tab", "ctrl+shift+tab"],
  },
  "view.navigate.focusNextTab": {
    icon: "fas fa-chevron-right",
    title: "Focus Next Tab",
    text: "Navigate to the next tab",
    shortcuts: ["alt+tab", "ctrl+tab"],
  },
  "view.navigate.closeTab": {
    icon: "fas fa-xmark",
    title: "Close Tab",
    text: "Close this tab",
    shortcuts: ["mod+w", "ctrl+w"],
  },
  "view.navigate.closeOtherTabs": {
    icon: "fas fa-xmark",
    title: "Close Other Tabs",
    text: "Close all other tabs",
  },
  "view.navigate.focusPreviousFrame": {
    icon: "fas fa-chevrons-left",
    title: "Focus Previous Frame",
    text: "Navigate to the previous frame",
    shortcuts: ["ctrl+mod+shift+space"],
  },
  "view.navigate.focusNextFrame": {
    icon: "fas fa-chevrons-right",
    title: "Focus Next Frame",
    text: "Navigate to the next frame",
    shortcuts: ["shift+mod+space"],
  },
  "view.navigate.closeFrame": {
    icon: "fas fa-xmark",
    title: "Close Frame",
    text: "Close this frame",
    shortcuts: ["mod+shift+w"],
  },
  "view.navigate.focusPreviousSplit": {
    icon: "fas fa-chevron-up",
    title: "Focus Previous Split",
    text: "Navigate to the previous split",
    shortcuts: ["ctrl+shift+up", "ctrl+shift+left"],
  },
  "view.navigate.focusNextSplit": {
    icon: "fas fa-chevron-down",
    title: "Focus Next Split",
    text: "Navigate to the next split",
    shortcuts: ["ctrl+shift+down", "ctrl+shift+right"],
  },
  "view.navigate.closeSplit": {
    icon: "fas fa-xmark",
    title: "Close Split",
    text: "Close this split",
  },
  // layout
  "view.layout.splitUp": {
    icon: "fas fa-reflect-vertical",
    title: "Split Up",
    text: "Split this view vertically (new split above)",
  },
  "view.layout.splitDown": {
    icon: "fas fa-reflect-vertical",
    title: "Split Down",
    text: "Split this view vertically (new split below)",
  },
  "view.layout.splitLeft": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Left",
    text: "Split this view horizontally (new split left)",
  },
  "view.layout.splitRight": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Right",
    text: "Split this view horizontally (new split right)",
  },
  "view.layout.pinSplit": {
    type: "toggle",
    icon: "fas fa-thumbtack",
    title: "Pin Split",
    text: "Pin this split to an absolute size",
  },
  "view.space.resetDefault": {
    title: "Restore Default Space",
    text: "Reset the space to the default layout",
    icon: "fas fa-galaxy",
    command: () => {
      if (pkg.value == null || space.value == null) return;
      const tx = canvas.tx();
      clearSpace(tx, canvas.graph, space.value);
      createDesktopDefaultSpace(tx, space.value);
    },
  },
});
