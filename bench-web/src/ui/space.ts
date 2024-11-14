import { HELPER_VIEW_TYPES, ROOT_VIEW_TYPES, toCamelName } from "@/language/const";
import { getContainingFlow } from "@/language/flow";
import { isDescendantOf, type NodeKey, type ReadNodeGraph } from "@/language/graph";
import { cloneNode, generateNodeName, makeNode, NodeIn, packSubnode, unpackSubnode } from "@/language/node";
import { getOrderKey, updateOrder } from "@/language/order";
import { makeEdit, newChangeId, TransactionOptions, type Transaction } from "@/language/transaction";
import { unpackBuiltinObject } from "@/language/value";
import {
  BlockType,
  ChangeCategory,
  DESCENDANT_NODE_TYPES,
  HelpAspect,
  HubAspect,
  IconData,
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
  makeStruct,
  propertyInfo,
  toNodeRef,
  toNodeRefOneOf,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type AnyNodeReferenceData,
  type SomeNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { inspectionBasePtr, inspectionPtr } from "@/system/space";
import type { SplitAnchor } from "@/ui/drag";
import { toIconMaybe } from "@/ui/icon";
import { DEFAULT_ORIENTATION, splitBox } from "@/ui/layout";
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
import { getElement, isFocusableElement } from "@/utils/element";
import { generateOrderKey, generateOrderKeys } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
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
export type ViewIn = Partial<NodeIn<NodeType.VIEW>> &
  Required<Pick<NodeIn<NodeType.VIEW>, "type">> & { icon?: string | IconData };

type OpenViewOptions = {
  predicate?: (view: ViewData) => boolean;
  ifPresent?: "duplicate" | "focus" | "upsertAndFocus";
  props?: ViewIn;
};

const activeElement = useActiveElement();
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
  highlightedPtr = ref<SomeNodeReferenceData | null>(null);

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
    watch(activeElement, () => {
      if (
        activeElement.value != null &&
        activeElement.value !== document.body &&
        // NOTE: Chromium consider scrollable containers to be focusable elements, so ignore those.
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
      if (e.target != null && e.target != activeElement.value && !isOutsideView(e.target as HTMLElement)) {
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
  }

  /** Whether the node is currently inspected. */
  isInspected(node: AnyNodeData | SomeNodeReferenceData | null | undefined): boolean {
    return (
      node != null &&
      inspectionPtr.value != null &&
      (inspectionPtr.value.id == node.id || inspectionPtr.value.ck == (node as any).ck)
    );
  }

  /** Whether the node is currently highlighted. */
  isHighlighted(node: AnyNodeData | SomeNodeReferenceData | null | undefined): boolean {
    return (
      node != null &&
      this.highlightedPtr.value != null &&
      (this.highlightedPtr.value.id == node.id || this.highlightedPtr.value.ck == (node as any).ck)
    );
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
  ): SomeNodeReferenceData | null {
    if (element != null) {
      const nodePtr = view?.exposed?.mapToNode?.(element);
      if (nodePtr != null) return nodePtr;
    }
    const nodePtr = this.graph.getMaybe(getViewComponentPtrMaybe(view))?.nodePtr ?? (view.props as ViewProps).nodePtr;
    return unwrapProtoOneOf(nodePtr) ?? null;
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
  getNodeAt(el: HTMLElement | SVGElement): SomeNodeReferenceData | null {
    // traverse upwards until we find some node ptr or a component that gives us a node ptr
    while (el != null) {
      if (el.dataset["nodeId"] != null) {
        // annotated element
        const nodePtr = {
          metatype: ObjectType.NODE_REFERENCE,
          nodeType: Number(el.dataset["nodeType"]),
          id: el.dataset["nodeId"],
          ck: el.dataset["nodeCk"],
        };
        return nodePtr;
      } else if ((el as any).__viewComponent != null) {
        // view component
        const component = (el as any).__viewComponent as ViewComponent;
        if (component.exposed.mapToNode != null) {
          const nodePtr = component.exposed.mapToNode(el);
          if (nodePtr != null) return nodePtr;
        } else if (component.props.nodePtr?.oneofKind != null) {
          const nodePtr = unwrapProtoOneOf(component.props.nodePtr);
          if (nodePtr != null) return nodePtr;
        }
      }
      // up we go
      el = el.parentElement!;
    }
    return null;
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
    let nodePtr: SomeNodeReferenceData | null = null;
    for (const viewComponent of viewComponents) {
      nodePtr = this.getViewNodePtr(viewComponent, element as HTMLElement);
      if (nodePtr != null) break;
    }
    const keepInspectionInBase =
      getElement(element)?.closest?.("[data-keep-inspection-in-base-view]") != null &&
      inspectionBasePtr.value != null &&
      nodePtr != null &&
      isDescendantOf(this.graph, inspectionBasePtr.value, nodePtr);
    if (
      !keepInspectionInBase &&
      baseView != null &&
      !HELPER_VIEW_TYPES.has(baseView.type) &&
      nodePtr != null &&
      !this.isInspected(nodePtr)
    ) {
      this.inspect({ node: nodePtr, view: this.focusedViewPtr.value! });
    }
    if (
      !keepInspectionInBase &&
      this.focusedViewPtr.value != null &&
      focusedView?.focus?.nodesPtr[0]?.id != nodePtr?.id
    ) {
      this.focusInGraph({ view: this.focusedViewPtr.value, focus: makeSelectionMaybe(nodePtr) });
    }
  }

  /** Inspects the given node */
  inspect(inspect: {
    node: AnyNodeData | AnyNodeReferenceData;
    view: SomeView | ViewComponent | ComponentInstance<any> | HTMLElement | SVGElement;
    focusInspector?: boolean;
  }): void {
    log.trace("canvas.inspect", inspect);

    const nodePtr = toPlainNodeRef(inspect.node as AnyNodeData);
    let view: SomeView | null;
    if (isNodeRef(inspect.view) || isNode(inspect.view)) {
      view = inspect.view as SomeView;
    } else {
      view = this.findViewData(inspect.view);
    }
    if (view == null) throw new Error(`no view for ${inspect.view}`);
    const viewAncestors = this.graph.getAncestors(view, { metatypes: [NodeType.VIEW], includeSelf: true });
    const rootViewIdx = viewAncestors.findIndex((v) => ROOT_VIEW_TYPES.has(v.type));
    const baseNodePtr = unwrapProtoOneOf(viewAncestors[rootViewIdx - 1]?.nodePtr);
    if (inspectionPtr.value?.id != nodePtr.id || inspectionBasePtr.value?.id != baseNodePtr?.id) {
      const space = this.graph.getOrError(this.spacePtr.value!);
      this.tx().update(space, { inspectionPtr: nodePtr, basePtr: baseNodePtr }, { debounce: "short" });
    }

    // open inspector
    if (inspect.focusInspector) {
      this.addView({ type: ViewType.INSPECT }, { ifPresent: "focus" });
    }
  }

  /**
   * Focus the given view or node absolutely in the graph and in the component.
   * Also updates inspection to that node if re-focusing a view.
   * */
  focus(
    focus: (
      | { node: TypedNodeReferenceData<NodeType.VIEW> | ViewData }
      | { node: AnyNodeReferenceData | AnyNodeData; view: SomeView }
    ) & { anchor?: FocusAnchor | NodeReferenceData; ignoreInspection?: boolean },
  ) {
    log.trace("canvas.focus", focus);
    focus.node = this.graph.getOrError({ id: focus.node.id }); // 'refresh' node in graph since it may have moved
    const nodeType = isNodeRef(focus.node) ? (focus.node as NodeReferenceData).nodeType : focus.node.metatype;

    if (nodeType == NodeType.VIEW && this.isInSpace(focus.node)) {
      // focus as a view in canvas
      const node = focus.node as ViewData | TypedNodeReferenceData<NodeType.VIEW>;

      // recover view & inspection from views' 'focus' down from focused view
      let viewData = this.getViewData(node);
      while (!HELPER_VIEW_TYPES.has(viewData?.type!) && (viewData?.focus?.nodesPtr?.length ?? 0) > 0) {
        if (viewData!.focus!.nodesPtr.some((v) => v.nodeType == NodeType.VIEW)) {
          const viewPtr = viewData!.focus!.nodesPtr.find((v) => v.nodeType == NodeType.VIEW);
          viewData = viewPtr != null ? this.getViewData(viewPtr) : null;
        } else if (!focus.ignoreInspection) {
          // auto-inspect what was previously focused inside this view
          const node = viewData!.focus!.nodesPtr[0];
          this.inspect({ node, view: viewData! });
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
          log.debug("canvas.focusFailed", focus, { viewData, component });
        }
      });
    } else if ("view" in focus) {
      // focus as a general node in the given view
      const node = focus.node as AnyNodeReferenceData | AnyNodeData;
      // focus in graph & then in component
      this.focusInGraph({ view: focus.view, focus: makeSelection([node]) });
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
  focusInGraph(focus: { view: SomeView; focus?: SelectionData; parent?: SomeView; resetDown?: boolean }) {
    const tx = this.tx();

    // focus the given selection within the view
    if (focus.focus != null) {
      const view = this.getViewData(focus.view)!;
      if (!deepValueEquals(view.focus, focus.focus)) tx.update(view, { focus: focus.focus }, { debounce: "short" });
    }

    // focus every 'child' in its 'parent' up to space root
    let child = this.getViewData(focus.view);
    if (child == null) throw new Error(`no view in graph for ${focus.view}`);
    let parent: ViewData | SpaceData | null = this.getViewData(focus.parent ?? child.parentPtr!);
    const updated = [];
    while (parent?.metatype == ObjectType.VIEW || parent?.metatype == ObjectType.SPACE) {
      const childFocus = makeSelection([child]);
      if (!deepValueEquals(parent.focus, childFocus)) {
        tx.update(parent, { focus: childFocus }, { debounce: "short" });
        updated.push(parent, { focus: childFocus });
      }
      child = parent as ViewData;
      parent = this.graph.getMaybe(child.parentPtr) as ViewData | SpaceData | null;
    }

    // reset focus 'down' from view
    if (focus.resetDown) {
      const descendants = this.graph.getDescendants(child, { metatypes: [NodeType.VIEW] });
      descendants
        .filter((v) => v.focus != null)
        .forEach((v) => tx.update(v, { focus: undefined }, { debounce: "short" }));
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
      log.debug("canvas.focusInComponent.error", { view, viewPtr, error });
      return false;
    }

    // if no anchor is given, try to use existing focus state
    if (anchor == null) {
      if (viewData?.focus?.nodesPtr?.some((n) => n.nodeType == NodeType.VIEW)) {
        const child = this.getViewData(viewData.focus.nodesPtr.find((n) => n.nodeType == NodeType.VIEW)!);
        if (child != null) {
          // if we have a focus state we must use it, even if it didn't actually focus in the component
          //  (so we 'pretend' there was focus in the component by calling onComponentFocused directly)
          if (!this.focusInComponent(child)) this.onComponentFocused(component);
          return true;
        }
      } else {
        anchor = viewData?.focus?.nodesPtr?.[0];
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

  /** Restores component focus to the currently absolutely focused element if possible. */
  restoreComponentFocus(): boolean {
    if (this.spacePtr.value == null) throw new Error("no current space");
    const space = this.graph.get(this.spacePtr.value);
    if ((space?.focus?.nodesPtr?.length ?? 0) > 0) {
      const view = this.getViewData(space!.focus!.nodesPtr[0]);
      if (view != null) {
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

    // NOTE :Architecture: 'self'/'id' shouldn't change, right, so no need to watch in Canvas.registerView?
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
    // base view
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
    const baseViewRef: Ref<ViewData | null> | null = (base as any)?.__selfViewRef ?? null;

    //
    // state (on demand)
    //

    function getState(viewId?: string): Partial<Record<string, any> | undefined> {
      const subviewPacked = (baseViewRef?.value?.subviewsPacked as any)?.[viewId ?? componentId];
      if (subviewPacked == null) return undefined;
      return unpackBuiltinObject(subviewPacked, ObjectType.VIEW);
    }

    function getChildState(viewId: string): Partial<Record<string, any> | undefined> {
      return getState(componentId + "." + viewId);
    }

    function update(update: Partial<NodeIn<any>>, options?: TransactionOptions) {
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
          const key = update.type.toString();
          if ((instance?.props?.subnodePacked as any)?.[key] != null) {
            const subnode = unpackSubnode(NodeType.VIEW, update.type, instance!.props.subnodePacked as any);
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

    return { getState, getChildState, update };
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

  /** Finds a view with properties exactly like the criteria */
  findView(like: {
    type: ViewType;
    nodePtr: AnyNodeReferenceData | undefined;
    predicate?: (view: ViewData) => boolean;
  }): ViewData | null {
    if (Object.keys(like).length == 0) return null;
    if (this.spacePtr.value == null) return null;
    const views = this.graph.getDescendants(this.spacePtr.value, { metatypes: [NodeType.VIEW] });
    const match = views.find((v) => {
      if (like.type != null && v.type != like.type) return false;
      if (like.nodePtr != null && unwrapProtoOneOf(v.nodePtr)?.id != like.nodePtr?.id) return false;
      if (like.predicate != null && !like.predicate(v)) return false;
      return true;
    });
    return match ?? null;
  }

  /** Add a new view to the canvas at the current root.  */
  addView(view: ViewIn, options?: OpenViewOptions) {
    const tx = this.tx();
    const existing = this.findView({
      type: view.type,
      nodePtr: unwrapProtoOneOf(view.nodePtr),
      predicate: options?.predicate,
    });

    if (existing == null || options?.ifPresent == null || options?.ifPresent == "duplicate") {
      // find root
      const parent =
        this.views.find((v) => v.type == ViewType.TAB || v.type == ViewType.HISTORY) ??
        this.views.find((v) => v.type == ViewType.WINDOW || v.type == ViewType.SPLIT) ??
        this.space.value!;
      log.debug("canvas.addView.create", view, { existing, options, parent });
      let siblings = this.graph.getChildren(parent, NodeType.VIEW);
      const change = this.tx().with({ change: { key: newChangeId() } });

      // prune history from current focused tab
      if (isNode(parent, NodeType.VIEW) && parent.type == ViewType.HISTORY) {
        const focusedId = parent.focus?.nodesPtr[0].id;
        const focusedTabIdx = siblings.findIndex((tab) => tab.id == focusedId) ?? -1;
        for (let i = focusedTabIdx + 1; i < siblings.length; i++) {
          change.delete(siblings[i]);
        }
        siblings = siblings.slice(0, focusedTabIdx + 1);
      }

      // create & focus
      const newView = makeNode({
        ...view,
        metatype: NodeType.VIEW,
        packagePtr: parent.packagePtr,
        orderKey: generateOrderKey(siblings[-1]?.orderKey ?? null, null),
        parentPtr: toPlainNodeRef(parent),
        icon: toIconMaybe(view.icon),
      });
      if ((view.name ?? "").length == 0) {
        newView.name = generateNodeName(
          newView,
          this.graph.getDescendants(this.spacePtr.value!, { metatypes: [NodeType.VIEW] }),
        );
      }
      change.create(newView);
      this.focus({ node: newView });
      return newView;
    } else if (options?.ifPresent == "focus") {
      log.trace("canvas.addView.focus", view, { existing, options });
      this.focus({ node: existing });
      return existing;
    } else if (options?.ifPresent == "upsertAndFocus") {
      log.trace("canvas.addView.upsertAndFocus", view, { existing, options });
      if (!deepValueEquals(existing.icon, view.icon) || !deepValueEquals(existing.focus, view.focus)) {
        tx.update(existing, { icon: toIconMaybe(view.icon), focus: view.focus }, { debounce: "tick" });
      }
      this.focus({ node: existing });
      return existing;
    } else {
      assertNever(options?.ifPresent);
    }
  }

  /** Upserts a view in the canvas (addView with upsertAndFocus). */
  upsertView(view: ViewIn) {
    this.addView(view, { ifPresent: "upsertAndFocus" });
  }

  /**
   * Goes to the given node, whatever that means. Unlike addView, this upserts the view by default.
   * If it's a view node, we focus it in the space graph (it must exist).
   * If it's a regular node, we find or open an appropriate view for it and focus that somehow.
   */
  goToNode(
    node: AnyNodeData | NodeReferenceData | null,
    options?: { graph?: ReadNodeGraph; skipSelf?: boolean } & OpenViewOptions,
  ) {
    const nodePtr = isNodeRef(node) ? node : toNodeRef(node as AnyNodeData);
    const graph = options?.graph ?? this.graph;
    log.debug("canvas.goToNode", node);
    if (graph.has(nodePtr)) {
      // reload from graph
      node = graph.get(nodePtr);
    }
    if (node == null) {
      // not found
      throw new Error(`node not found in ${describeScope(graph.scope)}: ${describeNode(nodePtr)}`);
    } else if (nodePtr.nodeType == NodeType.VIEW && this.isInSpace(node)) {
      // just focus directly
      this.focus({ node: nodePtr as ViewData | TypedNodeReferenceData<NodeType.VIEW> });
    } else if (
      (isNode(node, NodeType.BLOCK) && node.type == BlockType.FLOW) ||
      isNode(node, NodeType.STEP) ||
      isNode(node, NodeType.PIPE) ||
      (isNode(node, NodeType.FIELD) && getContainingFlow(graph, node) != null)
    ) {
      // open as flow
      const containingFlow = getContainingFlow(graph, node);
      if (!containingFlow) throw new Error(`no containing flow for: ${describeNode(node)}`);
      const view = this.addView(
        {
          type: ViewType.FLOW,
          nodePtr: toNodeRefOneOf(containingFlow),
          focus: makeSelection([node]),
          ...options?.props,
        },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: nodePtr, view });
    } else if ((isNode(node, NodeType.BLOCK) && node.type == BlockType.DATABASE) || isNode(node, NodeType.RECORD)) {
      // open as database
      let view: ViewData;
      if (isNode(node, NodeType.RECORD)) {
        const block = graph.get(node.blockPtr!);
        if (block == null) throw new Error(`no containing block for record: ${describeNode(node)}`);
        view = this.addView(
          {
            type: ViewType.DATABASE,
            nodePtr: toNodeRefOneOf(block),
            focus: makeSelection([node]),
            ...options?.props,
          },
          { ifPresent: "upsertAndFocus", ...options },
        );
      } else {
        view = this.addView(
          { type: ViewType.DATABASE, nodePtr: toNodeRefOneOf(node), ...options?.props },
          { ifPresent: "upsertAndFocus", ...options },
        );
      }
      this.inspect({ node: nodePtr, view });
    } else if (isNode(node, NodeType.BLOCK) && node.type == BlockType.VIEW) {
      // open as view
      this.addView(
        { type: ViewType.VIEW, nodePtr: toNodeRefOneOf(nodePtr), ...options?.props },
        { ifPresent: "upsertAndFocus", ...options },
      );
    } else if (isNode(node, NodeType.BLOCK) || DESCENDANT_NODE_TYPES[NodeType.BLOCK].includes(nodePtr.nodeType)) {
      // open generic block in containing page
      const containingPage = graph
        .getAncestors(nodePtr, { metatypes: [NodeType.BLOCK], includeSelf: !options?.skipSelf })
        .find((n) => n.type == BlockType.PAGE);
      if (!containingPage) throw new Error(`in-block has no containing page block: ${describeNode(node)}`);
      const view = this.addView(
        {
          type: ViewType.PAGE,
          nodePtr: toNodeRefOneOf(containingPage),
          focus: makeSelection(nodePtr),
          ...options?.props,
        },
        { ifPresent: "upsertAndFocus", ...options },
      );
      this.inspect({ node: nodePtr, view });
    } else if (isNode(node, NodeType.RUN)) {
      const view = this.addView(
        {
          type: ViewType.RUN,
          nodePtr: toNodeRefOneOf(node),
          title: `Run ${node.id!.replace(/[^\w]/g, "").slice(20)}`,
          ...options?.props,
        },
        { ifPresent: "upsertAndFocus", ...options },
      );
    } else {
      throw new Error(`cannot go to node: ${describeNode(node)}`);
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
  ) {
    log.debug("canvas.move", { graph, ...move });
    const { self, child, anchor, referenceId } = move;
    const tx = this.tx();
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
      tx.move(child, { parentPtr: toPlainNodeRef(self) }, { debounce: "tick" });
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
    log.debug("canvas.split", { graph, ...split });
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
        parentPtr: toPlainNodeRef(split),
        size: undefined,
        orderKey: isOrderFlipped ? "a0" : "a1",
      });

      // and a new tab wrapper
      const childWrapper = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
        parentPtr: toPlainNodeRef(split),
        packagePtr: parent.packagePtr,
        name: `${parent.name}${toCasing(anchor.toUpperCase(), Casing.CAMEL)}`,
        orderKey: isOrderFlipped ? "a1" : "a0",
      });
      tx.create(childWrapper);
      tx.move(child, {
        parentPtr: toPlainNodeRef(childWrapper),
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
      tx.move(child, { parentPtr: toPlainNodeRef(newSplitParent), size: undefined, orderKey: "a0" });
      tx.update(parent, { size: halfSize });
    }
    this.cleanupRootViews(graph, graph.get(child.parentPtr!) as ViewData);
  }
}

function makeMainWindow(space: SpaceData, tx: Transaction): ViewData {
  const main = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.WINDOW,
    parentPtr: toPlainNodeRef(space),
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
  tx.update(space, { focus: undefined, inspectionPtr: undefined }, { debounce: "short" });
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
      parentPtr: toPlainNodeRef(parent),
      packagePtr: root.packagePtr,
    });
    if (viewsByName[view.name] != null) throw new Error(`duplicate view name: ${view.name}`);
    viewsByName[view.name] = view;
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
      type: ViewType.HUB,
      name: "Side",
      size: makeStruct({ metatype: StructType.RECTANGLE, width: 320 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 280, maxWidth: 500 }),
      subnode: { aspect: HubAspect.SOURCE },
    },
    {
      type: ViewType.HISTORY,
      name: "Main",
      size: makeStruct({ metatype: StructType.RECTANGLE, widthRelative: 1000 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 600 }),
    },
    {
      type: ViewType.HELP,
      name: "Detail",
      orientation: Orientation.VERTICAL,
      size: makeStruct({ metatype: StructType.RECTANGLE, width: 500 }),
      constraint: makeStruct({ metatype: StructType.RECTANGLE_CONSTRAINT, minWidth: 400, maxWidth: 800 }),
      subnode: { aspect: HelpAspect.INSPECT },
    },
  ]);
  return { primary: layout.viewsByName["Primary"] };
}
