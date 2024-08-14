import { HELPER_VIEW_TYPES, PAGE_BLOCK_TYPES, ROOT_VIEW_TYPES, toCamelName } from "@/language/const";
import { isDescendantOf, type NodeKey, type ReadNodeGraph } from "@/language/graph";
import { cloneNode, generateNodeName, makeNode } from "@/language/node";
import { getOrderKey, updateOrder } from "@/language/order";
import { packProtoJson, unpackProtoJson, type DebounceLevel, type Transaction } from "@/language/transaction";
import { isProtoJson, packBuiltinObject, packBuiltinObjectJson, unpackBuiltinObject } from "@/language/value";
import {
  Anchor,
  DESCENDANT_NODE_TYPES,
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  SelectionData,
  SelectionKind,
  SelectionTarget,
  SpaceData,
  StructType,
  TreeViewPreset,
  ViewData,
  ViewType,
  type AnyNodeData,
  type AnyTypeMapping,
} from "@/proto/wire";
import {
  describeNode,
  isNodeRef,
  makeStruct,
  toNodeRef,
  toNodeRefOneOf,
  toPlainNodeRef,
  typeNodeReferenceMaybe,
  unwrapProtoOneOf,
  type AnyNodeReferenceData,
  type SomeNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { canvas, inspectionBasePtr, inspectionPtr } from "@/system/space";
import type { SplitAnchor } from "@/ui/drag";
import { toIconMaybe } from "@/ui/icon";
import { DEFAULT_ORIENTATION, splitBox } from "@/ui/layout";
import { getElement, isFocusableElement } from "@/utils/element";
import { generateOrderKey, generateOrderKeys } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { IS_DEV, isDeveloperMode } from "@/utils/globals";
import { log } from "@/utils/log";
import { computedValue, deepValueEquals } from "@/utils/ref";
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
  toRef,
  triggerRef,
  watch,
  type ComponentInstance,
  type MaybeRef,
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
  predicate?: (view: ViewData) => boolean;
  where?: "currentFrame" | "bestFrame";
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
  tx: () => Transaction; // for when we're not given a transaction to work with (e.g. browser events)
  viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});

  // absolutely focused views/components (focused from the top down)
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up
  focusedViewPtr: Ref<TypedNodeReferenceData<NodeType.VIEW> | null> = shallowRef(null);

  constructor(
    spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>,
    graph: ReadNodeGraph,
    txFactory: () => Transaction,
  ) {
    this.spacePtr = spacePtr;
    this.graph = graph;
    this.tx = txFactory;

    // respond to 'unmanaged' input from browser
    // active element
    watch(activeElement, () => {
      if (
        activeElement.value != null &&
        activeElement.value !== document.body &&
        // NOTE: Chrome thinks that scrollable containers are a focusable element, so ignore those.
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
  }

  /** Whether there are any active views here */
  get isEmpty() {
    return this.viewRefsById.value == null || Object.keys(this.viewRefsById.value).length === 0;
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
  getViewNodePtr(view: ViewComponent, element?: HTMLElement | ViewComponent | null): SomeNodeReferenceData | null {
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

  /** Updates our internal focus state in response to a browser event */
  private onComponentFocused(element: ViewComponent | HTMLElement | null) {
    const component = element instanceof HTMLElement ? findViewComponentUp(element) : element;

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
      getElement(element)?.closest?.("[data-keep-inspection-in-base]") != null &&
      inspectionBasePtr.value != null &&
      nodePtr != null &&
      isDescendantOf(this.graph, inspectionBasePtr.value, nodePtr);
    if (
      !keepInspectionInBase &&
      baseView != null &&
      !HELPER_VIEW_TYPES.has(baseView.type) &&
      nodePtr != null &&
      nodePtr.id != inspectionPtr.value?.id
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
  inspect(inspect: { node: AnyNodeData | AnyNodeReferenceData; view: SomeView; focusInspector?: boolean }): void {
    log.trace("canvas.inspect", inspect);

    const nodePtr = toPlainNodeRef(inspect.node as AnyNodeData);
    const viewAncestors = this.graph.getAncestors(inspect.view, { metatypes: [NodeType.VIEW], includeSelf: true });
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
    const nodeType = isNodeRef(focus.node) ? (focus.node as NodeReferenceData).type : focus.node.metatype;

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
      } else if ((el as any).__viewComponent == null) {
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

  /** Gets the containing root view (or self, if any) for a view */
  getRootView(view: ViewData): ViewData {
    if (ROOT_VIEW_TYPES.has(view.type)) return view;
    const ancestors = this.graph.getAncestors(view, { metatypes: [NodeType.VIEW], includeSelf: true });
    return ancestors.find((v) => ROOT_VIEW_TYPES.has(v.type)) ?? view;
  }

  /** Gets the current subroot view ('lowest' focused view within a root) */
  get focusedRoot(): ViewData | null {
    if (this.focusedViewPtr.value == null) return null;
    if (this.spacePtr.value == null) return null;

    // traverse focused view up until we find a root
    const view = this.graph.get(this.focusedViewPtr.value);
    if (view == null) return null;
    return this.getRootView(view);
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
  addView(view: ViewDataIn, options?: OpenViewOptions) {
    const tx = this.tx();
    const existing = this.findView({
      type: view.type,
      nodePtr: unwrapProtoOneOf(view.nodePtr),
      predicate: options?.predicate,
    });
    const currentFrame = this.focusedRoot;
    log.debug("canvas.addView", view, { existing, options, focusedRoot: currentFrame });

    if (existing == null || options?.ifPresent == null || options?.ifPresent == "duplicate") {
      // find/make root
      let parent: ViewData | null = null;
      if (options?.where == null || options?.where == "currentFrame") {
        parent = currentFrame;
      } else if (options?.where == "bestFrame" && currentFrame != null) {
        // NOTE :UX: improve how we choose the best frame for a new view
        // if there's an existing frame containing the same view type, use that
        const similarView = this.views.find((v) => v.type == view.type);
        if (similarView != null) {
          parent = this.getRootView(similarView);
        }
      } else {
        throw new Error(`unexpected where: ${options?.where}`);
      }
      if (parent == null) {
        // no parent so far, just use current
        parent = currentFrame;
      }
      if (parent == null) {
        // no parent at all, reset space (either we're in a local empty space or it got messed up somehow)
        log.debug("canvas.repairCanvas", this.spacePtr.value);
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
        parentPtr: toPlainNodeRef(parent),
        icon: toIconMaybe(view.icon),
      });
      if ((view.name ?? "").length == 0)
        newView.name = generateNodeName(
          NodeType.VIEW,
          this.graph.getDescendants(this.spacePtr.value!, { metatypes: [NodeType.VIEW] }),
          newView.type,
        );
      tx.create(newView);
      this.focus({ node: newView });
      return newView;
    } else if (options?.ifPresent == "focus") {
      this.focus({ node: existing });
      return existing;
    } else if (options?.ifPresent == "upsertAndFocus") {
      tx.update(existing, { icon: toIconMaybe(view.icon), focus: view.focus });
      this.focus({ node: existing });
      return existing;
    } else {
      assertNever(options?.ifPresent);
    }
  }

  /** Upserts a view in the canvas (addView with upsertAndFocus). */
  upsertView(view: ViewDataIn) {
    this.addView(view, { ifPresent: "upsertAndFocus" });
  }

  /**
   * Goes to the given node, whatever that means. Unlike addView, this upserts the view by default.
   * If it's a view node, we focus it in the space graph (it must exist).
   * If it's a regular node, we find or open an appropriate view for it and focus that somehow.
   */
  goToNode(
    node: AnyNodeData | NodeReferenceData,
    options?: { graph?: ReadNodeGraph; skipSelf?: boolean } & OpenViewOptions,
  ) {
    const nodeRef = isNodeRef(node) ? node : toNodeRef(node as AnyNodeData);
    log.debug("canvas.goToNode", node);
    const tx = this.tx();
    if (nodeRef.type == NodeType.VIEW && this.isInSpace(node)) {
      // just focus directly
      this.focus({ node: nodeRef as ViewData | TypedNodeReferenceData<NodeType.VIEW> });
    } else {
      // find or create appropriate view
      const graph = options?.graph ?? this.graph;
      if (nodeRef.type == NodeType.BLOCK || DESCENDANT_NODE_TYPES[NodeType.BLOCK].includes(nodeRef.type)) {
        const containingPage = graph
          .getAncestors(nodeRef, { metatypes: [NodeType.BLOCK], includeSelf: !options?.skipSelf })
          .find((n) => PAGE_BLOCK_TYPES.includes(n.type));
        if (!containingPage) throw new Error(`in-block has no containing page block: ${describeNode(node)}`);
        const view = this.addView(
          {
            type: ViewType.PAGE,
            nodePtr: toNodeRefOneOf(containingPage),
            focus: makeSelection(nodeRef),
          },
          { ifPresent: "upsertAndFocus", ...options },
        );
        this.inspect({ node: nodeRef, view });
      } else {
        throw new Error(`cannot go to node: ${describeNode(node)}`);
      }
    }
  }

  /**
   * Removes the given view from the space graph, taking care to clean up.
   */
  removeView(graph: ReadNodeGraph, view: ViewData) {
    log.debug("canvas.remove", view);
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
      graph.get({ type: NodeType.VIEW, id: view.parentPtr!.id })?.type == ViewType.SPLIT
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

// NOTE :Cleanup: defining space/canvas layouts is a bit cumbersome

type ViewLayoutIn = {
  type: ViewType;
  children?: ViewLayoutIn[];
} & Partial<ViewData>;

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
    const name = viewIn.name ?? generateNodeName(NodeType.VIEW, viewsByType[viewIn.type] ?? [], viewIn.type);
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

/** Sets up a minimal empty space with one root tab */
export function createEmptySpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  const window = makeMainWindow(space, tx);
  const layout = makeLayout(tx, window, [{ type: ViewType.TAB, name: "Primary", children: [] }]);
  return { primary: layout.viewsByName["Primary"] };
}

/** Creates the default three-side canvas */
export function createDesktopDefaultSpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  const window = makeMainWindow(space, tx);
  const layout = makeLayout(tx, window, [
    {
      type: ViewType.TAB,
      name: "Side",
      orientation: Orientation.VERTICAL,
      size: makeStruct({ metatype: StructType.BOX, width: 320 }),
      children: [
        {
          type: ViewType.TREE,
          title: "Explore",
          valuePacked: packBuiltinObjectJson({ metatype: ObjectType.TREE_VIEW_STATE, preset: TreeViewPreset.EXPLORE }),
        },
      ],
    },
    {
      type: ViewType.TAB,
      name: "Primary",
      size: makeStruct({ metatype: StructType.BOX, widthRelative: 1500 }),
    },
    {
      type: ViewType.TAB,
      name: "Secondary",
      size: makeStruct({ metatype: StructType.BOX, widthRelative: 700 }),
      children: [
        { type: ViewType.INSPECT },
        { type: ViewType.FEED, name: "Logs1", title: "Logs" },
        { type: ViewType.START },
      ],
    },
  ]);
  return { primary: layout.viewsByName["Primary"] };
}

/** Creates the advanced three-side double vertical split canvas */
export function createDesktopAdvancedSpace(tx: Transaction, space: SpaceData): { primary: ViewData } {
  return createDesktopDefaultSpace(tx, space); // no special space yet
}

export function makeSelection(
  nodes: AnyNodeData | AnyNodeReferenceData | (AnyNodeData | AnyNodeReferenceData)[],
): SelectionData {
  nodes = Array.isArray(nodes) ? nodes : [nodes];
  return {
    metatype: ObjectType.SELECTION,
    target: SelectionTarget.NODE,
    kind: SelectionKind.LIST,
    nodesPtr: nodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData))),
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
    ...(selection ?? { metatype: ObjectType.SELECTION, target: SelectionTarget.NODE, kind: SelectionKind.LIST }),
    nodesPtr: [...(selection?.nodesPtr ?? []), ...nodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData)))],
  };
}

export function collapseSelection(selection: SelectionData, nodes: (AnyNodeData | NodeReferenceData)[]): SelectionData {
  return {
    ...selection,
    nodesPtr: selection.nodesPtr.filter((n) => !nodes.some((m) => m.id == n.id)),
  };
}

export function useExpansion(options: {
  graph: ReadNodeGraph;
  tx: () => Transaction;
  self?: Ref<AnyNodeReferenceData | null | undefined>;
  props: Pick<ViewData, "expansion">;
  emit: (event: string, ...args: any[]) => void;
  isDefaultExpanded?: MaybeRef<boolean | undefined>;
  isExclusive?: boolean;
}) {
  const isDefaultExpandedRef = toRef(options.isDefaultExpanded) as Ref<boolean>;
  const expandedNodesById = computedValue(() => {
    const expanded: Record<string, NodeReferenceData> = {};
    for (const node of options.props.expansion?.nodesPtr ?? []) {
      expanded[node.id!] = node;
    }
    return expanded;
  });

  function toggleExpanded(node: AnyNodeData | AnyNodeReferenceData) {
    if (isDefaultExpandedRef.value) return; // nothing to do

    let newExpansion: SelectionData | null;
    if (isExpanded(node)) {
      newExpansion = collapseSelection(options.props.expansion!, [node]);
    } else {
      if (options.isExclusive) {
        newExpansion = makeSelection([node]);
      } else {
        newExpansion = expandSelection(options.props.expansion, [node]);
      }
    }
    if (options.self?.value != null) {
      const self = options.graph.getOrError(options.self.value!);
      canvas.tx().update(self, { expansion: newExpansion }, { debounce: "tick" });
    } else {
      options.emit("update:self", { expansion: newExpansion });
    }
  }

  function isExpanded(node: { id?: string; ck?: string }): boolean {
    return isDefaultExpandedRef.value || expandedNodesById.value[node.id!] != null;
  }

  return { toggleExpanded, isExpanded };
}

/**
 * Use the typed state in the View.value of a builtin view type.
 **/
export function useViewState<T extends ObjectType>(use: {
  selfPtr: Ref<TypedNodeReferenceData<NodeType.VIEW> | undefined | null>;
  graph: ReadNodeGraph;
  stateType: T;
  props: Pick<ViewData, "valuePacked">;
  emit: (event: string, ...args: any[]) => void;
}) {
  const state = computedValue(() => {
    if (use.props.valuePacked == null) {
      return { metatype: use.stateType } as AnyTypeMapping[T];
    } else {
      // we don't pack proto json structs inside proto json structs,
      //  so while valuePacked should be a proto struct (as per the type) it may not be (see :ProtoStructMapping)
      const valuePacked = isProtoJson(use.props.valuePacked)
        ? unpackProtoJson(use.props.valuePacked)
        : use.props.valuePacked;
      const unpacked = unpackBuiltinObject(valuePacked, use.stateType);
      return unpacked;
    }
  });

  function updateState(value: Partial<AnyTypeMapping[T]>, options?: { debounce?: DebounceLevel }) {
    const valuePacked = packStateUpdate(value);
    if (use.selfPtr.value != null) {
      const tx = canvas.tx();
      const self = use.graph.getOrError(use.selfPtr.value);
      tx.update(self, { valuePacked: packProtoJson(valuePacked) }, options);
    } else {
      use.emit("update:self", { valuePacked: packProtoJson(valuePacked) });
    }
  }

  function packStateUpdate(value: Partial<AnyTypeMapping[T]>) {
    const newState = { ...state.value, ...value } as AnyTypeMapping[T];
    const valuePacked = packBuiltinObject(newState);
    return valuePacked;
  }

  function useStateProp<P extends keyof AnyTypeMapping[T]>(
    prop: P,
    defaultValue: AnyTypeMapping[T][P],
    options?: { debounce?: DebounceLevel },
  ): Ref<Required<AnyTypeMapping[T]>[P]>;
  function useStateProp<P extends keyof AnyTypeMapping[T]>(prop: P): Ref<AnyTypeMapping[T][P] | undefined>;
  function useStateProp<P extends keyof AnyTypeMapping[T]>(
    prop: P,
    defaultValue?: AnyTypeMapping[T][P],
    options?: { debounce?: DebounceLevel },
  ): Ref<AnyTypeMapping[T][P]> {
    return computed({
      get: () => (state.value?.[prop] ?? defaultValue) as any,
      set: (value: AnyTypeMapping[T][P]) => updateState({ [prop]: value } as any, options),
    });
  }

  return { state, updateState, packStateUpdate, useStateProp };
}
