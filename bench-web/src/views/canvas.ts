import {
  Anchor,
  BenchType,
  DESCENDANT_NODE_TYPES,
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  PrimitiveType,
  SelectionData,
  SelectionKind,
  SpaceData,
  StructType,
  TypeInfoData,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import {
  copyNode,
  describeNode,
  getNodeType,
  makeNode,
  makeStruct,
  toNodeReference,
  typeNodeReferenceMaybe,
  type AnyNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import type { Connection } from "@/system/connection";
import { isDescendantOf, type NodeKey, type ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE, toIconMaybe } from "@/system/icon";
import {
  NODE_VIEW_TYPES,
  HELPER_VIEW_TYPES,
  ROOT_VIEW_TYPES,
  generateNodeName,
  getOrderKey,
  updateOrder,
} from "@/system/lang";
import { inspectionBasePtr, inspectionPtr } from "@/system/space";
import type { Transaction } from "@/system/transaction";
import type { SplitAnchor } from "@/utils/drag";
import { getElement, isFocusableElement } from "@/utils/element";
import { generateOrderKey } from "@/utils/fractional";
import { IS_DEV, isDeveloperMode } from "@/utils/globals";
import { DEFAULT_ORIENTATION, splitBox } from "@/utils/layout";
import { log } from "@/utils/log";
import { deepValueEquals, toValueRef } from "@/utils/ref";
import { Casing, toCasing } from "@/utils/string";
import { getViewTypeByComponentName, type FocusAnchor, type ViewComponent, type ViewProps } from "@/views/common";
import { useActiveElement, useEventListener, type MaybeElement } from "@vueuse/core";
import {
  computed,
  getCurrentInstance,
  nextTick,
  onBeforeUnmount,
  onMounted,
  onUpdated,
  shallowRef,
  triggerRef,
  watch,
  type ComponentInstance,
  type Ref,
} from "vue";

export const DEFAULT_BAR_POSITION = Anchor.TOP;
export const DEFAULT_HEADER_HEIGHT = 36;
export const DEFAULT_MIN_WIDTH = 320;
export const DEFAULT_MAX_WIDTH = 800;
export const DEFAULT_PADDING_X = 20;

export function getVueComponentType(component: ComponentInstance<any>): string {
  return component.__name ?? (component as any).type.__name;
}

export function describeVueComponent(component: ComponentInstance<any>): string {
  const id = (component as any).exposed?.self?.value?.id ?? (component as any).exposed?.id?.value;
  return `${getVueComponentType(component)}:${id}`;
}

/** Gets a top down 'path' of a vue component (like Space:id->Split:id->Tabbed:id->Button:id) */
export function describeVueComponentPath(component: ComponentInstance<any>): string {
  const components = collectViewComponentsUp(component).reverse();
  return components.map((c) => getVueComponentType(c) + ":" + getViewComponentId(c)).join("->");
}

export function isVueComponent(component: ComponentInstance<any>): boolean {
  return (component as any).uid != null;
}

export function isVueInstanceOf(component: ComponentInstance<any>, type: string | { __name?: string }): boolean {
  const componentType = (component as any).type;
  return typeof type === "string" ? componentType.__name === type : componentType === type;
}

export function isViewComponent(component: ComponentInstance<any>): component is ViewComponent {
  return (component as any).exposed?.self != null || (component as any).exposed?.id != null;
}

export function isIdentifiedViewComponent(
  component: ComponentInstance<any>,
): component is ViewComponent & { exposed: { self: Ref<NodeReferenceData> } } {
  return (component as any).exposed?.self?.value != null;
}

export function isViewComponentIn(component: ViewComponent, viewTypes: Set<ViewType>): boolean {
  const componentType = getVueComponentType(component);
  const viewType = getViewTypeByComponentName(componentType);
  if (viewType == null) throw new Error(`no view type for component: ${componentType}`);
  return viewTypes.has(viewType);
}

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds the closest ViewComponent ancestor. */
export function findViewComponentUp(
  el: HTMLElement | ComponentInstance<any>,
  where?: (component: ViewComponent) => boolean,
): ViewComponent | null {
  while (el != null) {
    if (el instanceof HTMLElement) {
      // first find vue component
      if ((el as any).__viewComponent != null) el = (el as any).__viewComponent;
      else el = el.parentElement!;
    } else {
      if (where == null || where(el)) return el;
      else el = el.parent;
    }
  }
  return null;
}

/** Collect all view components from the given component upwards (inclusive) */
export function collectViewComponentsUp(componentOrEl: ComponentInstance<any> | HTMLElement): ViewComponent[] {
  let component = componentOrEl instanceof HTMLElement ? findViewComponentUp(componentOrEl) : componentOrEl;
  const components = [];
  while (component != null) {
    if (isViewComponent(component)) components.push(component);
    component = component.parent;
  }
  return components;
}

/**
 * Gets all child View components of a given component in DOM order.
 * Walks the DOM descendants until the first layer of child components.
 * */
export function getViewComponentChildren(instance: ComponentInstance<any>): ViewComponent[] {
  const elements = [instance.subTree.el];
  const components = [];

  // traverse the DOM
  while (elements.length > 0) {
    const el = elements.pop()!;
    for (const child of el.children) {
      if (child instanceof HTMLElement) {
        const component = (child as any).__viewComponent as ComponentInstance<any> | null;
        if (component != null && component !== instance && isViewComponent(component)) components.push(component);
        else elements.push(child);
      }
    }
  }

  return components;
}

function getViewComponentPtrMaybe(
  component: ViewComponent | null | undefined,
): TypedNodeReferenceData<NodeType.VIEW> | null {
  return typeNodeReferenceMaybe(NodeType.VIEW, component?.exposed.self?.value ?? null);
}

const activeElement = useActiveElement();

/** Traverses the DOM up to check if any element is marked as outside any view */
function isOutsideView(el: HTMLElement): boolean {
  while (el != null) {
    if (el.hasAttribute("data-outside-view")) return true;
    el = el.parentElement!;
  }
  return false;
}

export type SomeView = NodeReferenceData | ViewData;
export type ViewDataIn = Partial<Omit<ViewData, "metatype" | "icon">> &
  Pick<ViewData, "type"> & { icon?: string | IconData };

type OpenViewOptions = {
  where?: "currentRoot" | "nextFrameRoot";
  ifPresent?: "duplicate" | "focus" | "upsertAndFocus";
};

/**
 * Canvas, manager and helper for linking Views, their Vue components, and their HTML elements in a Space.
 * Some of our View components may not have an associated View, so we track them with a derived id.
 * NOTE: ViewCanvas is effectively a global singleton (currently).
 */
export class ViewCanvas {
  spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction; // for when we're not given a transaction to work with (e.g. browser events)
  viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});

  // absolutely focused views/components (focused from the top down)
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up
  focusedViewPtr: Ref<TypedNodeReferenceData<NodeType.VIEW> | null> = shallowRef(null);
  focusedView: Ref<ViewData | null>;

  constructor(
    spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>,
    graph: ReadNodeGraph,
    txFactory: () => Transaction,
  ) {
    this.spacePtr = spacePtr;
    this.graph = graph;
    this.txFactory = txFactory;

    // respond to 'unmanaged' input from browser
    // active element
    watch(activeElement, () => {
      if (
        activeElement.value != null &&
        activeElement.value !== document.body &&
        // NOTE: Chrome pretends that scrollable containers are focusable element, so ignore those.
        //  (Otherwise we would get confused because activeElement change comes after mousedown event,
        //   and if the mousedown'ed target was the next higher container will trigger later, changing focus)
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

    this.focusedView = toValueRef(this.graph.getRef(this.focusedViewPtr));
  }

  /** Whether there are any active views here */
  get isEmpty() {
    return this.viewRefsById.value == null || Object.keys(this.viewRefsById.value).length === 0;
  }

  /** Gets the absolutely focused view components in bottom up order */
  get focusedViewComponents(): ViewComponent[] {
    return Object.values(this.focusedViewComponentsById.value);
  }

  /** Gets the node ptr of the given view */
  getViewNodePtr(nodeView: ViewComponent, element?: HTMLElement | ViewComponent | null) {
    if (element != null) {
      const nodePtr = nodeView?.exposed?.mapToNode?.(element);
      if (nodePtr != null) return nodePtr;
    }
    return this.graph.getMaybe(getViewComponentPtrMaybe(nodeView))?.nodePtr ?? (nodeView.props as any).nodePtr;
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

  /** Updates our internal focus state in response to a browser event */
  private onComponentFocused(element: ViewComponent | HTMLElement | null) {
    const component = element instanceof HTMLElement ? findViewComponentUp(element) : element;
    if (this.focusedViewComponent.value === component) return;

    // update component focus state
    if (component == null) {
      // reset
      this.focusedViewComponent.value = null;
      this.focusedViewComponentsById.value = {};
      this.focusedViewPtr.value = null;
    }

    // refresh
    const tx = this.txFactory();

    // update focus
    this.focusedViewComponent.value = component;
    const componentsById: Record<string, ViewComponent> = {};
    const viewComponents = collectViewComponentsUp(component);
    viewComponents.forEach((c) => {
      componentsById[getViewComponentId(c)] = c;
    });
    this.focusedViewComponentsById.value = componentsById;
    this.focusedViewPtr.value = getViewComponentPtrMaybe(viewComponents.find(isIdentifiedViewComponent));

    // update root/inspection
    const rootViewComponentIdx = viewComponents.findIndex((v) => isViewComponentIn(v, ROOT_VIEW_TYPES));
    const baseView = this.graph.getMaybe(getViewComponentPtrMaybe(viewComponents[rootViewComponentIdx - 1]));
    const containingNodeView = viewComponents.find((v) => isViewComponentIn(v, NODE_VIEW_TYPES));
    const linkedNodePtr = containingNodeView && element ? this.getViewNodePtr(containingNodeView, element) : null;
    const keepInspectionInBase =
      getElement(element)?.closest?.("[data-keep-inspection-in-base]") != null &&
      inspectionBasePtr.value != null &&
      linkedNodePtr != null &&
      isDescendantOf(this.graph, inspectionBasePtr.value, linkedNodePtr);
    if (
      !keepInspectionInBase &&
      baseView != null &&
      !HELPER_VIEW_TYPES.has(baseView.type) &&
      linkedNodePtr != null &&
      linkedNodePtr != inspectionPtr.value?.id
    ) {
      this.inspect(tx, { node: linkedNodePtr, view: this.focusedViewPtr.value! });
    }
    if (
      !keepInspectionInBase &&
      this.focusedViewPtr.value != null &&
      this.focusedView.value?.focus?.nodesPtr[0]?.id != linkedNodePtr?.id
    ) {
      this.focusInGraph(tx, { view: this.focusedViewPtr.value, focus: makeSelectionMaybe(linkedNodePtr) });
    }
  }

  /** Inspects the given node */
  inspect(
    tx: Transaction,
    inspect: { node: AnyNodeData | AnyNodeReferenceData; view: SomeView; focusInspector?: boolean },
  ): void {
    log.trace("canvas.inspect", inspect);

    const nodePtr = toNodeReference(inspect.node as AnyNodeData);
    const viewAncestors = this.graph.getAncestors(inspect.view, { metatypes: [NodeType.VIEW], includeSelf: true });
    const rootViewIdx = viewAncestors.findIndex((v) => ROOT_VIEW_TYPES.has(v.type));
    const baseNodePtr = viewAncestors[rootViewIdx - 1]?.nodePtr;
    if (inspectionPtr.value?.id != nodePtr.id || inspectionBasePtr.value?.id != baseNodePtr?.id) {
      const space = this.graph.getOrError(this.spacePtr.value!);
      tx.update(space, { inspectionPtr: nodePtr, basePtr: baseNodePtr }, { debounce: "short" });
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
    tx: Transaction,
    focus: (
      | { node: TypedNodeReferenceData<NodeType.VIEW> | ViewData }
      | { node: AnyNodeReferenceData | AnyNodeData; view: SomeView }
    ) & { anchor?: FocusAnchor | NodeReferenceData; ignoreInspection?: boolean },
  ) {
    log.trace("canvas.focus", focus);
    const nodeType = getNodeType(focus.node);

    if (nodeType == NodeType.VIEW && this.isInSpace(focus.node)) {
      // focus as a view in canvas
      const node = focus.node as ViewData | TypedNodeReferenceData<NodeType.VIEW>;

      // recover view & inspection from views' 'focus' down from focused view
      let viewData = this.getViewData(node);
      while (!HELPER_VIEW_TYPES.has(viewData?.type!) && (viewData?.focus?.nodesPtr?.length ?? 0) > 0) {
        if (viewData!.focus!.nodesPtr.some((v) => v.type == NodeType.VIEW)) {
          const viewPtr = viewData!.focus!.nodesPtr.find((v) => v.type == NodeType.VIEW);
          viewData = viewPtr != null ? this.getViewData(viewPtr) : null;
        } else if (!focus.ignoreInspection) {
          // auto-inspect what was previously focused inside this view
          const node = viewData!.focus!.nodesPtr[0];
          this.inspect(tx, { node, view: viewData! });
          break;
        }
      }
      if (viewData == null) throw new Error(`no view data for ${describeNode(node)}`);

      // focus in graph & then in component
      this.focusInGraph(tx, { ...focus, view: viewData });
      nextTick(() => {
        const focused = this.focusInComponent(node, focus.anchor);
        if (!focused) {
          const component = this.getViewComponent(viewData!.id!);
          if (component != null) this.onComponentFocused(component);
          log.warn("canvas.focusFailed", focus, { viewData, component });
        }
      });
    } else if ("view" in focus) {
      // focus as a general node in the given view
      const node = focus.node as AnyNodeReferenceData | AnyNodeData;
      // focus in graph & then in component
      this.focusInGraph(tx, { view: focus.view, focus: makeSelection([node]) });
      if (!focus.ignoreInspection) {
        this.inspect(tx, { node, view: focus.view });
      }
      const component = this.getViewComponent(focus.view.id!);
      if (component != null) this.focusInComponent(component, focus.anchor);
    } else {
      throw new Error(`unexpected focus: ${focus}`);
    }
  }

  /** Focuses the given view absolutely in the graph. */
  focusInGraph(
    tx: Transaction,
    focus: { view: SomeView; focus?: SelectionData; parent?: SomeView; resetDown?: boolean },
  ) {
    // focus the given selection within the view
    if (focus.focus != null) {
      const view = this.getViewData(focus.view)!;
      if (!deepValueEquals(view.focus, focus.focus)) tx.update(view, { focus: focus.focus }, { debounce: "short" });
    }

    // focus every 'child' in its 'parent' up to space root
    let child = this.getViewData(focus.view);
    if (child == null) throw new Error(`no view in graph for ${focus.view}`);
    let parent: ViewData | SpaceData | null = this.getViewData(focus.parent ?? child.parentPtr!);
    while (parent?.metatype == ObjectType.VIEW || parent?.metatype == ObjectType.SPACE) {
      const childFocus = makeSelection([child]);
      if (!deepValueEquals(parent.focus, childFocus)) tx.update(parent, { focus: childFocus }, { debounce: "short" });
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
      const viewPtr = viewData ?? (view as ViewComponent).exposed?.self.value;
      throw new Error(`no component for view: ${viewPtr != null ? describeNode(viewPtr) : getVueComponentType(view)}`);
    }

    // if no anchor is given, try to use existing focus state
    if (anchor == null) {
      if (viewData?.focus?.nodesPtr?.some((n) => n.type == NodeType.VIEW)) {
        const child = this.getViewData(viewData.focus.nodesPtr.find((n) => n.type == NodeType.VIEW)!);
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
  registerView(self: Ref<NodeReferenceData | undefined>, id?: Ref<string>): ViewComponent {
    const instance = getCurrentInstance() as ViewComponent | null;
    if (instance == null) throw new Error("no current Vue instance");

    // mark element with component
    function markEl() {
      // NOTE: we enforce that el must be a single element for all Views with a lint rule
      //  (unfortunately this doesn't prevent comments from forcing the root into a #text node during development, so we error below)
      const el = (instance as any).vnode.el as HTMLElement | null;
      if (!el) {
        log.warn("canvas.missingEl", getVueComponentType(instance), instance);
      } else {
        (el as any).__viewComponent = instance;
        if ("dataset" in el) el.dataset.view = "true";
        else throw new Error(`invalid root element in ${getVueComponentType(instance)}: ${el}`);
      }
    }
    onMounted(markEl);
    onUpdated(markEl);

    // register
    // NOTE @Cleanup: 'self'/'id' should never change, so no need to watch in Canvas.registerView?
    let oldComponentId: string | null = null;
    watch(
      () => self.value?.id ?? id?.value ?? null,
      () => {
        if (oldComponentId != null && this.viewRefsById.value[oldComponentId] === instance) {
          delete this.viewRefsById.value[oldComponentId];
        }

        const componentId = self.value?.id ?? id?.value!;
        const existingComponent = this.viewRefsById.value[componentId];
        if (existingComponent != null && (IS_DEV || isDeveloperMode.value)) {
          // NOTE: checking for duplicate components only works reliably on next tick
          //  (because we may be registering a new component before the old component is unmounted)
          nextTick(() => {
            if (
              (existingComponent as any).vnode?.el != null &&
              document.body.contains((existingComponent as any).vnode.el) // is this really the fastest way to check if it's still mounted?
            ) {
              const thisPath = describeVueComponentPath(instance);
              const existingPath = describeVueComponentPath(existingComponent);
              throw new Error(`duplicate components for id ${componentId}: ${thisPath} vs ${existingPath}`);
            }
          });
        }
        this.viewRefsById.value[componentId] = instance;
        triggerRef(this.viewRefsById);
        oldComponentId = componentId;
      },
      { immediate: true },
    );
    // unregister
    onBeforeUnmount(() => {
      // should always be true, but maybe errored
      if (oldComponentId != null && this.viewRefsById.value[oldComponentId] === instance) {
        delete this.viewRefsById.value[oldComponentId];
        triggerRef(this.viewRefsById);
      }
    });
    return instance;
  }

  /** Gets the current root view ('lowest' focused root view) */
  get focusedRoot(): ViewData | null {
    if (this.focusedViewPtr.value == null) return null;
    if (this.spacePtr.value == null) return null;

    // traverse focused view up until we find a root
    let view = this.graph.get(this.focusedViewPtr.value);
    if (view == null) return null;
    while (view.parentPtr?.id != null) {
      if (ROOT_VIEW_TYPES.has(view.type)) return view;
      view = this.graph.get(view.parentPtr) as ViewData;
    }
    return null; // not found
  }

  isInSpace(node: NodeKey<any>): boolean {
    return isDescendantOf(this.graph, node, this.spacePtr.value!);
  }

  /** Gets all the open frames (direct children of Window views, not reactive) */
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

  /** Finds a view with properties exactly like the criteria */
  findView(like: Pick<ViewData, "type" | "nodePtr">): ViewData | null {
    if (Object.keys(like).length == 0) return null;
    if (this.spacePtr.value == null) return null;
    const views = this.graph.getDescendants(this.spacePtr.value, { metatypes: [NodeType.VIEW] });
    const match = views.find((v) => {
      if (like.type != null && v.type != like.type) return false;
      if (like.nodePtr != null && v.nodePtr?.id != like.nodePtr.id) return false;
      return true;
    });
    return match ?? null;
  }

  /** Add a new view to the canvas at the current root.  */
  addView(view: ViewDataIn, options?: OpenViewOptions) {
    const tx = this.txFactory();
    const existing = this.findView({ type: view.type, nodePtr: view.nodePtr });
    log.debug("canvas.addView", view, { existing, options, focusedRoot: this.focusedRoot });

    if (existing == null || options?.ifPresent == null || options?.ifPresent == "duplicate") {
      // find/make root
      let parent: ViewData | null = null;
      if (options?.where == null || options?.where == "currentRoot") {
        parent = this.focusedRoot;
      } else if (options?.where == "nextFrameRoot" && this.focusedRoot != null) {
        // find root window and root tab below it
        const ancestors = this.graph.getAncestors(this.focusedRoot, { metatypes: [NodeType.VIEW] });
        const rootSplit = ancestors[ancestors.length - 2];
        if (rootSplit != null) {
          const rootSplitSiblings = this.graph.getChildren(ancestors[ancestors.length - 1], NodeType.VIEW);
          const nextSplit = rootSplitSiblings[rootSplitSiblings.findIndex((n) => n.id == rootSplit.id) + 1];
          if (nextSplit != null) parent = nextSplit;
        }
      } else {
        throw new Error(`unexpected where: ${options?.where}`);
      }
      if (parent == null) {
        // no parent so far, just use current
        parent = this.focusedRoot;
      }
      if (parent == null) {
        // no parent at all, reset space (got messed up somehow)
        log.info("canvas.repairCanvas", this.spacePtr.value);
        const space = this.graph.getOrError(this.spacePtr.value!);
        parent = createEmptySpace(tx, space).primary;
      }

      // create & focus
      const rootChildren = this.graph.getChildren(parent, NodeType.VIEW);
      const newView = makeNode({
        ...view,
        metatype: NodeType.VIEW,
        packagePtr: parent.packagePtr,
        orderKey: generateOrderKey(rootChildren[-1]?.orderKey ?? null, null),
        parentPtr: toNodeReference(parent),
        icon: toIconMaybe(view.icon),
      });
      if ((view.name ?? "").length == 0)
        newView.name = generateNodeName(
          NodeType.VIEW,
          this.graph.getDescendants(this.spacePtr.value!, { metatypes: [NodeType.VIEW] }),
          newView.type,
        );
      tx.create(newView);
      this.focus(tx, { node: newView });
      return newView;
    } else if (options?.ifPresent == "focus") {
      this.focus(tx, { node: existing });
      return existing;
    } else if (options?.ifPresent == "upsertAndFocus") {
      tx.update(existing, { icon: toIconMaybe(view.icon), focus: view.focus });
      this.focus(tx, { node: existing });
      return existing;
    } else {
      throw new Error(`unexpected ifPresent: ${options?.ifPresent}`);
    }
  }

  /** Upserts a view in the canvas (addView with upsertAndFocus). */
  upsertView(view: ViewDataIn) {
    this.addView(view, { ifPresent: "upsertAndFocus" });
  }

  /**
   * Goes to the given node, whatever that means. Unlike addView, this upserts the view by default.
   * If it's a view node, we focus it in the space graph (it must exist).
   * If it's a regular node, we find or open an appropriate view for it and focus accordingly.
   */
  goToNode(
    node: AnyNodeData | NodeReferenceData,
    options?: { graph?: ReadNodeGraph; skipSelf?: boolean } & OpenViewOptions,
  ) {
    const nodeRef =
      node.metatype == ObjectType.NODE_REFERENCE ? (node as NodeReferenceData) : toNodeReference(node as AnyNodeData);
    log.debug("canvas.goToNode", node);
    const tx = this.txFactory();
    if (nodeRef.type == NodeType.VIEW && this.isInSpace(node)) {
      // just focus directly
      this.focus(tx, { node: nodeRef as ViewData | TypedNodeReferenceData<NodeType.VIEW> });
    } else {
      // find or create appropriate view
      const graph = options?.graph ?? this.graph;
      if (nodeRef.type == NodeType.BLOCK || DESCENDANT_NODE_TYPES[NodeType.BLOCK].includes(nodeRef.type)) {
        const containingPage = graph
          .getAncestors(nodeRef, { metatypes: [NodeType.BLOCK], includeSelf: !options?.skipSelf })
          .find((n) => n.isPage);
        if (!containingPage) throw new Error(`in-block has no containing page block: ${describeNode(node)}`);
        const view = this.addView(
          {
            type: ViewType.PAGE,
            nodePtr: toNodeReference(containingPage),
            focus: makeSelection(nodeRef),
          },
          { ifPresent: "upsertAndFocus", ...options },
        );
        this.inspect(tx, { node: nodeRef, view });
      } else {
        throw new Error(`cannot go to node: ${describeNode(node)}`);
      }
    }
  }

  /**
   * Removes the given view from the space graph, taking care to clean up.
   */
  removeView(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
    log.debug("canvas.remove", view);
    const parent = graph.get(view.parentPtr!) as ViewData;
    tx.delete(view);
    this.cleanupRootViews(tx, graph, parent);
  }

  /**
   * Cleanup previously split (sub-)root views that are no longer needed.
   * NOTE: right now we only close sub root views because it's annoying to have your layout change because you accidentally close a tab.
   *  (and the 'layout' is usually your root splits)
   */
  cleanupRootViews(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
    if (!ROOT_VIEW_TYPES.has(view.type)) return;
    if (
      graph.getChildren(view, NodeType.VIEW).length == 0 &&
      graph.getChildren(view.parentPtr!, NodeType.VIEW).length > 1 &&
      graph.get({ type: NodeType.VIEW, id: view.parentPtr!.id })?.type == ViewType.SPLIT
    ) {
      // NOTE :UX: should we re-distribute space if cleaning up after a split?
      this.removeView(tx, graph, view);
    }
  }

  /**
   * Adds the given view into this view at the target/anchor.
   */
  moveView(
    tx: Transaction,
    graph: ReadNodeGraph,
    self: ViewData,
    child: ViewData,
    anchor: "start" | "end",
    referenceId: string | null,
  ) {
    log.debug("canvas.move", { self, child, anchor, referenceId });
    // move & update order
    if (child.id != referenceId) {
      updateOrder({
        tx,
        node: child,
        position: anchor == "start" ? "before" : "after",
        reference: referenceId,
        getNodes: () => graph.getChildren(self, NodeType.VIEW),
      });
    }
    if (child.parentPtr?.id != self.id) {
      tx.move(child, { parentPtr: toNodeReference(self) }, { debounce: "tick" });
      this.cleanupRootViews(tx, graph, graph.get(child.parentPtr!) as ViewData);
    }
  }

  /**
   * 'Splits' the 'parent' view to accomodate a new equally sized subview 'child' (at the anchor).
   * If we're already split alongside the given orientation, the child is added to the existing split.
   */
  splitView(
    tx: Transaction,
    graph: ReadNodeGraph,
    parent: ViewData,
    child: ViewData,
    anchor: Omit<SplitAnchor, "center">,
  ) {
    log.debug("canvas.split", { parent, child, anchor });

    // determine if we need a new split in the enclosing split view
    let split: ViewData | null = null;
    if (parent.type == ViewType.WINDOW) split = parent;
    else if (parent.parentPtr != null) split = graph.get(parent.parentPtr) as ViewData;
    if (split?.metatype != ObjectType.VIEW)
      throw new Error(
        `no enclosing split view: [parent=${ObjectType[parent.metatype]}, parent.type=${ViewType[parent.type]}]`,
      );
    const isHorizontal = anchor == "left" || anchor == "right";
    const orientation = isHorizontal ? Orientation.HORIZONTAL : Orientation.VERTICAL;
    const isOrderFlipped = anchor == "right" || anchor == "bottom";
    const needsNewSplit = (split.orientation ?? DEFAULT_ORIENTATION) != orientation;

    // duplicate child if it belongs to self
    if (child.parentPtr?.id == parent.id) {
      child = copyNode(child);
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
        parentPtr: toNodeReference(split),
        size: undefined,
        orderKey: isOrderFlipped ? "a0" : "a1",
      });

      // and a new tab wrapper
      const childWrapper = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
        parentPtr: toNodeReference(split),
        packagePtr: parent.packagePtr,
        name: `${parent.name}${toCasing(anchor.toUpperCase(), Casing.CAMEL)}`,
        orderKey: isOrderFlipped ? "a1" : "a0",
      });
      tx.create(childWrapper);
      tx.move(child, {
        parentPtr: toNodeReference(childWrapper),
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
          nodes: graph.getChildren(split, NodeType.VIEW),
          position: isOrderFlipped ? "after" : "before",
          reference: parent,
        }),
      });
      tx.create(newSplitParent);
      tx.move(child, {
        parentPtr: toNodeReference(newSplitParent),
        size: undefined,
        orderKey: "a0",
      });
      tx.update(parent, { size: halfSize });
    }
    this.cleanupRootViews(tx, graph, graph.get(child.parentPtr!) as ViewData);
  }
}

export function focusInElement(element: MaybeElement): boolean {
  while (element != null) {
    if (element instanceof HTMLElement || element instanceof SVGElement) {
      if (!isFocusableElement(element)) {
        return false; // don't try to magically find a focusable element, this shouldbe explicit
      } else {
        element.focus();
      }
      return true;
    } else if ("focus" in element) {
      const focusResult = (element as any).focus();
      if (focusResult === true || focusResult === undefined) return true;
      else element = focusResult;
    } else {
      return false;
    }
  }
  return false;
}

function makeMainWindow(space: SpaceData, tx: Transaction): ViewData {
  const main = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.WINDOW,
    parentPtr: toNodeReference(space),
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

// NOTE :Cleanup: defining space/canvas layouts is a bit cumbersome

/** Sets up a minimal empty space with one root tab */
export function createEmptySpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  const window = makeMainWindow(space, tx);
  const primary = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.TAB,
    parentPtr: toNodeReference(window),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Primary",
    title: "Primary",
  });
  tx.create(primary);
  return { primary };
}

/** Setups up the pro level three-side double split canvas */
export function createDesktopProSpace(
  tx: Transaction,
  space: SpaceData,
  options: { secondary: "split" | "side" | false } = { secondary: "split" },
): { side: ViewData; primary: ViewData; secondary: ViewData | null } {
  const window = makeMainWindow(space, tx);
  // root splits
  const side = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.SPLIT,
    parentPtr: toNodeReference(window),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Side",
    title: "Side",
    orientation: Orientation.VERTICAL,
    size: makeStruct({ metatype: StructType.BOX, width: 320 }),
  });
  const primary = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.TAB,
    parentPtr: toNodeReference(window),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Primary",
    title: "Primary",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1500 }),
  });

  // side
  const sideTop = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.TAB,
    parentPtr: toNodeReference(side),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Top",
    title: "Top",
  });
  const sideBottom = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.TAB,
    parentPtr: toNodeReference(side),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Bottom",
    title: "Bottom",
  });
  tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.EXPLORE,
    parentPtr: toNodeReference(sideTop),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Explore1",
    title: "Explore",
  });
  tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.OUTLINE,
    parentPtr: toNodeReference(sideBottom),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Outline1",
    title: "Outline",
  });

  // primary
  // ...?

  // secondary
  let secondary: ViewData | null = null;
  if (options.secondary == "split" || options.secondary == "side") {
    if (options.secondary == "split") {
      secondary = tx.create({
        metatype: NodeType.VIEW,
        type: ViewType.SPLIT,
        parentPtr: toNodeReference(window),
        packagePtr: space.packagePtr,
        orderKey: "a2",
        name: "Secondary",
        title: "Secondary",
        orientation: Orientation.VERTICAL,
        size: makeStruct({ metatype: StructType.BOX, widthRelative: 700 }),
      });
    } else {
      secondary = sideBottom;
    }
    const secondaryTop = tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.TAB,
      parentPtr: toNodeReference(secondary),
      packagePtr: space.packagePtr,
      orderKey: "a0",
      name: "Top",
      title: "Top",
    });
    const secondaryBottom = tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.TAB,
      parentPtr: toNodeReference(secondary),
      packagePtr: space.packagePtr,
      orderKey: "a1",
      name: "Bottom",
      title: "Bottom",
    });
    tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.INSPECT,
      parentPtr: toNodeReference(secondaryTop),
      packagePtr: space.packagePtr,
      orderKey: "a0",
      name: "Inspect1",
      title: "Inspect",
    });
    tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.START,
      parentPtr: toNodeReference(secondaryTop),
      packagePtr: space.packagePtr,
      orderKey: "a1",
      name: "Start1",
      title: "Start",
    });
    tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.CREATE,
      parentPtr: toNodeReference(secondaryBottom),
      packagePtr: space.packagePtr,
      orderKey: "a0",
      name: "Create1",
      title: "Create",
    });  
    tx.create({
      metatype: NodeType.VIEW,
      type: ViewType.FEED,
      parentPtr: toNodeReference(secondaryBottom),
      packagePtr: space.packagePtr,
      orderKey: "a1",
      name: "Logs1",
      title: "Logs",
    });
  }

  return { side, primary, secondary };
}

export function makeSelection(
  nodes: AnyNodeData | AnyNodeReferenceData | (AnyNodeData | AnyNodeReferenceData)[],
): SelectionData {
  nodes = Array.isArray(nodes) ? nodes : [nodes];
  return {
    metatype: ObjectType.SELECTION,
    kind: SelectionKind.LIST,
    nodesPtr: nodes.map((n) =>
      n.metatype == ObjectType.NODE_REFERENCE ? (n as NodeReferenceData) : toNodeReference(n as AnyNodeData),
    ),
  };
}

export function makeSelectionMaybe(
  nodes: AnyNodeData | AnyNodeReferenceData | (AnyNodeData | AnyNodeReferenceData)[] | null | undefined,
): SelectionData | undefined {
  if (nodes == null) return undefined;
  return makeSelection(Array.isArray(nodes) ? nodes : [nodes]);
}

export function expandSelection(
  selection: SelectionData | undefined | null,
  nodes: (AnyNodeData | NodeReferenceData)[],
): SelectionData {
  return {
    ...(selection ?? { metatype: ObjectType.SELECTION, kind: SelectionKind.LIST }),
    nodesPtr: [
      ...(selection?.nodesPtr ?? []),
      ...nodes.map((n) =>
        n.metatype == ObjectType.NODE_REFERENCE ? (n as NodeReferenceData) : toNodeReference(n as AnyNodeData),
      ),
    ],
  };
}

export function collapseSelection(selection: SelectionData, nodes: (AnyNodeData | NodeReferenceData)[]): SelectionData {
  return {
    ...selection,
    nodesPtr: selection.nodesPtr.filter(
      (n) =>
        !nodes.some((m) => {
          if (m.metatype == ObjectType.NODE_REFERENCE) return m.id == n.id;
          else return m.id == n.id;
        }),
    ),
  };
}

export function useExpansion(options: {
  graph: ReadNodeGraph;
  connection: Connection<any, any>;
  self: Ref<AnyNodeReferenceData | null | undefined>;
  isDefaultExpanded?: boolean;
}) {
  const selfNode = options.graph.getRef(options.self.value) as Ref<ViewData>;
  const expansion = computed(() => selfNode.value?.expansion);

  function toggleExpanded(node: AnyNodeData | AnyNodeReferenceData) {
    if (options.self.value == null) throw new Error("no self node");
    const selfNode = options.graph.getOrError(options.self.value) as ViewData;
    if (isExpanded(node)) {
      options.connection.tx.update(
        selfNode,
        { expansion: collapseSelection(selfNode.expansion!, [node]) },
        { debounce: "tick" },
      );
    } else if (!isExpanded(node)) {
      options.connection.tx.update(
        selfNode,
        { expansion: expandSelection(selfNode.expansion, [node]) },
        { debounce: "tick" },
      );
    }
  }

  function isExpanded(node: { id?: string; ck?: string }): boolean {
    return expansion.value?.nodesPtr?.some((n) => n.id == node.id) ?? false;
  }

  return { toggleExpanded, isExpanded: options.isDefaultExpanded ? () => true : isExpanded };
}
