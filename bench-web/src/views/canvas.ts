import {
  BenchType,
  IconData,
  NodeReferenceData,
  NodeType,
  Orientation,
  SelectionKind,
  SpaceData,
  StructType,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { copyNode, makeNode, makeStruct, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { NodeKey, ReadNodeGraph } from "@/system/graph";
import { toIconMaybe } from "@/system/icon";
import { ROOT_VIEW_COMPONENT_NAMES, ROOT_VIEW_TYPES, getOrderKey, updateOrderKey } from "@/system/lang";
import type { Transaction } from "@/system/transaction";
import type { SplitAnchor } from "@/utils/drag";
import { generateKeyBetween } from "@/utils/fractional";
import { DEFAULT_ORIENTATION, splitBox } from "@/utils/layout";
import { log } from "@/utils/log";
import type { ViewComponent } from "@/views";
import type { FocusAnchor } from "@/views/common";
import { useActiveElement, useEventListener } from "@vueuse/core";
import {
  computed,
  getCurrentInstance,
  nextTick,
  onBeforeUnmount,
  onMounted,
  shallowRef,
  triggerRef,
  watch,
  type ComponentInstance,
  type Ref,
} from "vue";

export function getVueComponentType(component: ComponentInstance<any>): string {
  return (component as any).type.__name;
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

export function isRootViewComponent(component: ComponentInstance<any>): boolean {
  return ROOT_VIEW_COMPONENT_NAMES.includes(getVueComponentType(component));
}

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds the closest ViewComponent ancestor. */
export function findViewComponent(
  el: HTMLElement | ComponentInstance<any>,
  where?: (component: ViewComponent) => boolean,
): ViewComponent | null {
  while (el != null) {
    if ((el as any).__viewComponent != null && (where == null || where((el as any).__viewComponent)))
      return (el as any).__viewComponent;
    else el = el.parentElement!;
  }
  return null;
}

/** Collect all view components from the given component upwards (inclusive) */
export function collectViewComponentsUp(componentOrEl: ComponentInstance<any> | HTMLElement): ViewComponent[] {
  let component = componentOrEl instanceof HTMLElement ? findViewComponent(componentOrEl) : componentOrEl;
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

/**
 * Canvas, manager and helper for linking Views, their Vue components, and their HTML elements.
 * Some of our View components may not have an associated View, so we track them with a derived id.
 * NOTE: ViewCanvas is effectively a global singleton (currently).
 */
export class ViewCanvas {
  spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction; // for when we're not given a transaction to work with (e.g. browser events)
  private viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});

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
    this.txFactory = txFactory;

    // respond to uncontrolled input from browser:
    // active element
    watch(activeElement, () => {
      if (activeElement.value != null && activeElement.value !== document.body && !isOutsideView(activeElement.value))
        this.onComponentFocused(activeElement.value);
    });
    // and 'focus' on any other element
    useEventListener(document, "mousedown", (e) => {
      if (e.target != null && e.target != activeElement.value && !isOutsideView(e.target as HTMLElement))
        this.onComponentFocused(e.target as HTMLElement);
    });
  }

  /** Updates our internal focus in response to a browser event */
  private onComponentFocused(element: ViewComponent | HTMLElement | null) {
    const component = element instanceof HTMLElement ? findViewComponent(element) : element;
    const wasDifferent = this.focusedViewComponent.value !== component;

    // update component focus state
    if (component == null) {
      this.focusedViewComponent.value = null;
      this.focusedViewComponentsById.value = {};
      this.focusedViewPtr.value = null;
    } else if (this.focusedViewComponent.value !== component) {
      this.focusedViewComponent.value = component;
      const componentsById: Record<string, ViewComponent> = {};
      collectViewComponentsUp(component).forEach((c) => {
        componentsById[getViewComponentId(c)] = c;
      });
      this.focusedViewComponentsById.value = componentsById;
      this.focusedViewPtr.value = (findViewComponent(component, isIdentifiedViewComponent)?.exposed.self?.value ??
        null) as TypedNodeReferenceData<NodeType.VIEW> | null;
    }

    // update graph focus state
    if (this.focusedViewPtr.value != null && wasDifferent) {
      this.focusInGraph(this.txFactory(), { view: this.focusedViewPtr.value });
    }
  }

  /** Gets the absolutely focused view components in bottom up order */
  get focusedViewComponents(): ViewComponent[] {
    return Object.values(this.focusedViewComponentsById.value);
  }

  /** Gets the view component for a certain view identity (self.id or anonymous id) */
  getViewComponent<T extends ViewComponent>(id: string): T | null {
    return this.viewRefsById.value[id] as T | null;
  }

  /** Resolve the view data */
  getViewData(view: SomeView): ViewData | null {
    if (view.metatype == BenchType.VIEW) return view as ViewData;
    else return this.graph.get(view as NodeKey<NodeType.VIEW>) as ViewData | null;
  }

  /** Focus the given view absolutely in the graph and in the component. */
  focus(
    tx: Transaction,
    focus: { view: SomeView; parent?: SomeView; anchor?: FocusAnchor | NodeReferenceData; hasBrowserFocus?: boolean },
  ) {
    log.debug("view.focus", focus);
    this.focusInGraph(tx, focus);
    if (!focus.hasBrowserFocus) nextTick(() => this.focusInComponent(focus.view, focus.anchor));
  }

  /** Focuses the given view absolutely in the graph. */
  focusInGraph(tx: Transaction, focus: { view: SomeView; parent?: SomeView; clearDown?: boolean }) {
    log.trace("view.focusInGraph", focus);

    // focus every 'child' in its 'parent' up to space root
    let child = this.getViewData(focus.view);
    if (child == null) throw new Error(`no view in graph for ${focus.view}`);
    let parent: ViewData | SpaceData | null = this.getViewData(focus.parent ?? child.parentPtr!);
    while (parent?.metatype == BenchType.VIEW || parent?.metatype == BenchType.SPACE) {
      tx.update({
        metatype: parent.metatype as unknown as NodeType.VIEW | NodeType.SPACE,
        id: parent.id,
        focus: {
          metatype: BenchType.SELECTION,
          kind: SelectionKind.LIST,
          nodesPtr: [toNodeReference(child)],
        },
      });
      child = parent as ViewData;
      parent = this.graph.getMaybe(child.parentPtr) as ViewData | SpaceData | null;
    }

    // reset focus 'down' from view
    if (focus.clearDown) {
      const descendants = this.graph.getDescendants(child, [NodeType.VIEW]) as ViewData[];
      descendants
        .filter((v) => v.focus != null)
        .forEach((v) => {
          tx.update({ ...v, metatype: NodeType.VIEW, focus: undefined });
        });
    }
  }

  /** Focus the first focusable component within the given view. */
  focusInComponent(view: SomeView | ViewComponent, anchor?: FocusAnchor | NodeReferenceData): boolean {
    log.trace("view.focusInComponent", view, anchor);

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
      throw new Error(`no component for view ${(view as any)?.id}`);
    }

    // if no anchor is given, try to use existing focus state
    if (anchor == null && (viewData?.focus?.nodesPtr?.length ?? 0) > 0) {
      const child = this.getViewData(viewData!.focus!.nodesPtr[0]);
      if (child != null && this.focusInComponent(child)) return true;
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
        // success
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
    log.trace("view.restoreComponentFocus", this.spacePtr.value);
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
  isFocusedAbsoluteRef(view: Ref<SomeView>): Ref<boolean> {
    return computed(() => this.isFocusedAbsolute(view.value));
  }

  /** Registers the current Vue component instance in the canvas with some View identity */
  registerView(self: Ref<NodeReferenceData | undefined>, id?: Ref<string>): ViewComponent {
    const instance = getCurrentInstance() as ViewComponent | null;
    if (instance == null) throw new Error("no current Vue instance");

    // mark element with component
    onMounted(() => {
      // we enforce that el must be a single element
      if ((instance as any).vnode.el == null) console.warn("canvas.missingEl", getVueComponentType(instance), instance);
      else (instance as any).vnode.el.__viewComponent = instance;
    });

    // register
    let oldComponentId: string | null = null;
    watch(
      [() => self.value?.id, () => id?.value],
      () => {
        if (oldComponentId != null && this.viewRefsById.value[oldComponentId] === instance)
          delete this.viewRefsById.value[oldComponentId];
        const componentId = self.value?.id ?? id?.value!;
        const existingComponent = this.viewRefsById.value[componentId];
        if (existingComponent != null) {
          // TODO :Robustness: check for duplicate component registration
          // Duplicate component ids happen for two reasons:
          //  1. When moving a view, the new component may be created before the old one is destroyed. This is fine.
          //  2. We messed up naming our own internal/anonymous components. This is bad.
          // Currently not sure how to distinguish these two cases.
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
      if (ROOT_VIEW_TYPES.includes(view.type)) return view;
      view = this.graph.get(view.parentPtr) as ViewData;
    }
    return null; // not found
  }

  /** Gets all the open windows (direct children of Windowed views) */
  get currentWindows(): ViewData[] {
    if (this.spacePtr.value == null) return [];
    const getWindows = (view: ViewData): ViewData[] => {
      if (view.type == ViewType.WINDOWED) {
        return this.graph.getChildren(view, NodeType.VIEW).flatMap(getWindows);
      } else {
        return [view];
      }
    };
    const windows = this.graph.getChildren(this.spacePtr.value, NodeType.VIEW).flatMap(getWindows);
    return windows;
  }

  /** Finds a view with properties exactly like the criteria */
  findView(like: Partial<ViewData>): ViewData | null {
    if (Object.keys(like).length == 0) return null;
    if (this.spacePtr.value == null) return null;
    const views = this.graph.getDescendants(this.spacePtr.value, [NodeType.VIEW]) as ViewData[];
    const match = views.find((v) => {
      // simple exact match every property
      for (const key in like) {
        if ((v as any)[key] != (like as any)[key]) return false;
      }
      return true;
    });
    return match ?? null;
  }

  /** Add a new view to the canvas at the current root.  */
  addView(
    view: ViewDataIn,
    options?: {
      where?: "currentRoot";
      ifPresent?: "duplicate" | "focus" | "upsertAndFocus";
    },
  ) {
    const tx = this.txFactory();
    const existing = this.findView({ type: view.type });

    if (existing == null || options?.ifPresent == null || options?.ifPresent == "duplicate") {
      // find/make root
      let root = this.focusedRoot ?? this.currentWindows[0];
      if (root == null) {
        // no root, reset space
        log.info("view.repair", this.spacePtr.value);
        const space = this.graph.get(this.spacePtr.value!)!;
        root = setupEmptyCanvas(tx, space).root;
      }

      // create & focus
      const rootChildren = this.graph.getChildren(root, NodeType.VIEW);
      const newView = makeNode({
        ...view,
        metatype: NodeType.VIEW,
        packagePtr: root.packagePtr,
        orderKey: generateKeyBetween(rootChildren[-1]?.orderKey ?? null, null),
        parentPtr: toNodeReference(root),
        icon: toIconMaybe(view.icon),
      });
      tx.create(newView);
      this.focus(tx, { view: newView });
    } else if (options?.ifPresent == "focus") {
      this.focus(tx, { view: existing });
    } else if (options?.ifPresent == "upsertAndFocus") {
      tx.update({
        id: existing.id,
        metatype: NodeType.VIEW,
        ...view,
        icon: toIconMaybe(view.icon),
      });
      this.focus(tx, { view: existing });
    }
  }

  /** Upserts a view in the canvas (addView with upsertAndFocus). */
  upsertView(view: ViewDataIn) {
    this.addView(view, { ifPresent: "upsertAndFocus" });
  }

  /**
   * Goes to the given node, whatever that means.
   * If it's a view node, we focus it in the space graph (it must exist).
   * If it's a regular node, we find or open an appropriate view for it and focus accordingly.
   */
  goToNode(node: NodeReferenceData, options?: {}) {
    if (node.type == NodeType.VIEW) {
      this.focus(this.txFactory(), { view: node });
    } else {
      throw new Error("not yet implemented");
    }
  }

  /**
   * Removes the given view from the space graph, taking care to clean up.
   */
  removeView(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
    log.debug("view.remove", view);
    const parent = graph.get(view.parentPtr!) as ViewData;
    tx.delete(view); // should soft delete?
    this.cleanupRootView(tx, graph, parent);
  }

  /**
   * Cleanup previously split root views that are no longer needed.
   */
  cleanupRootView(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
    if (!ROOT_VIEW_TYPES.includes(view.type)) return;
    if (
      graph.getChildren(view, NodeType.VIEW).length == 0 &&
      graph.getChildren(view.parentPtr!, NodeType.VIEW).length > 1
    ) {
      // TODO :UX: re-distribute space if cleaning up after a split
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
    log.debug("view.add", { self, child, anchor, referenceId });
    // move & update order
    if (child.id != referenceId) {
      updateOrderKey({
        tx,
        target: child,
        position: anchor == "start" ? "before" : "after",
        referenceId,
        nodes: () => graph.getChildren(self, NodeType.VIEW),
      });
    }
    if (child.parentPtr?.id != self.id) {
      tx.move({ ...child, parentPtr: toNodeReference(self) });
    }
    this.cleanupRootView(tx, graph, graph.get(child.parentPtr!) as ViewData);
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
    log.debug("view.split", { parent, child, anchor });

    // determine if we need a new split in the enclosing split view
    let split: ViewData;
    if (parent.type == ViewType.WINDOWED) split = parent;
    else if (parent.parentPtr != null) split = graph.get(parent.parentPtr) as ViewData;
    else
      throw new Error(
        `no enclosing split: [parent=${BenchType[parent.metatype]}, parent.type=${ViewType[parent.type]}]`,
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
        type: ViewType.WINDOWED,
        parentPtr: parent.parentPtr,
        packagePtr: parent.packagePtr,
        orderKey: parent.orderKey,
        size: parent.size,
        name: "Split",
        orientation,
      });
      tx.create(split);
      tx.move({ ...parent, parentPtr: toNodeReference(split) });
      tx.update({ ...parent, metatype: NodeType.VIEW, size: undefined, orderKey: isOrderFlipped ? "a0" : "a1" });

      // and a new tabbed wrapper
      const viewParent = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TABBED,
        parentPtr: toNodeReference(split),
        packagePtr: parent.packagePtr,
        orderKey: isOrderFlipped ? "a1" : "a0",
      });
      tx.create(viewParent);
      tx.move({ ...child, parentPtr: toNodeReference(viewParent) });
      tx.update({ ...child, metatype: NodeType.VIEW, size: undefined, orderKey: "a0" });
    } else {
      // 'split' size between self and child with a new tabbed wrapper
      const halfSize = splitBox(parent.size!);
      const newSplitParent = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TABBED,
        parentPtr: parent.parentPtr,
        packagePtr: parent.packagePtr,
        size: halfSize,
        orderKey: getOrderKey({
          nodes: graph.getChildren(split, NodeType.VIEW),
          position: isOrderFlipped ? "after" : "before",
          reference: parent,
        }),
      });
      tx.create(newSplitParent);
      tx.move({ ...child, parentPtr: toNodeReference(newSplitParent) });
      tx.update({ ...child, metatype: NodeType.VIEW, size: halfSize, orderKey: "a0" });
      tx.update({ ...parent, metatype: NodeType.VIEW, size: halfSize });
    }
    this.cleanupRootView(tx, graph, graph.get(child.parentPtr!) as ViewData);
  }
}

/** Sets up a minimal empty space with one root tabbed */
export function setupEmptyCanvas(tx: Transaction, space: SpaceData): { root: ViewData } {
  // nocheckin: add 'Windowed' node between 'Space' and 'View'?
  //  (for clarity so Views always have a parent View up to root, later to allow for multiple windows)
  const main = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.TABBED,
    parentPtr: toNodeReference(space),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Main Window",
    title: "Main Window",
  });
  tx.create(main);
  return { root: main };
}

/** Setups up the default three-window canvas */
export function setupDefaultCanvas(
  tx: Transaction,
  space: SpaceData,
): { side: ViewData; primary: ViewData; secondary: ViewData } {
  const side = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.TABBED,
    parentPtr: toNodeReference(space),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Side Window",
    title: "Side Window",
    size: makeStruct({ metatype: StructType.BOX, width: 300 }),
  });
  tx.create(side);
  const primary = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.TABBED,
    parentPtr: toNodeReference(space),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Primary Window",
    title: "Primary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1.5 }),
  });
  tx.create(primary);
  const secondary = makeNode({
    metatype: NodeType.VIEW,
    type: ViewType.TABBED,
    parentPtr: toNodeReference(space),
    packagePtr: space.packagePtr,
    orderKey: "a2",
    name: "Secondary Window",
    title: "Secondary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  tx.create(secondary);
  return { side, primary, secondary };
}
