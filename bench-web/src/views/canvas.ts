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
import {
  copyNode,
  describeNode,
  makeNode,
  makeStruct,
  toNodeReference,
  typeNodeReferenceMaybe,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
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

export function describeVueComponent(component: ComponentInstance<any>): string {
  const id = (component as any).exposed?.self?.value?.id ?? (component as any).exposed?.id?.value;
  return `${getVueComponentType(component)}:${id}`;
}

/** Gets a top down 'path' of a vue component (like Space:id->Split:id->Tabbed:id->Button:id) */
export function describeVueComponentPath(component: ComponentInstance<any>): string {
  const components = collectViewComponentsUp(component).reverse();
  return components.map((c) => getVueComponentType(c) + ":" + getViewComponentId(c)).join("->");
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
 * Canvas, manager and helper for linking Views, their Vue components, and their HTML elements in a Space.
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
      this.focusedViewPtr.value = typeNodeReferenceMaybe(
        NodeType.VIEW,
        findViewComponent(component, isIdentifiedViewComponent)?.exposed.self?.value ?? null,
      );
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
    log.debug("canvas.focus", focus);
    this.focusInGraph(tx, focus);
    if (!focus.hasBrowserFocus) nextTick(() => this.focusInComponent(focus.view, focus.anchor));
  }

  /** Focuses the given view absolutely in the graph. */
  focusInGraph(tx: Transaction, focus: { view: SomeView; parent?: SomeView; clearDown?: boolean }) {
    log.trace("canvas.focusInGraph", focus);

    // focus every 'child' in its 'parent' up to space root
    let child = this.getViewData(focus.view);
    if (child == null) throw new Error(`no view in graph for ${focus.view}`);
    let parent: ViewData | SpaceData | null = this.getViewData(focus.parent ?? child.parentPtr!);
    while (parent?.metatype == BenchType.VIEW || parent?.metatype == BenchType.SPACE) {
      tx.update(parent, {
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
      descendants.filter((v) => v.focus != null).forEach((v) => tx.update(v, { focus: undefined }));
    }
  }

  /** Focus the first focusable component within the given view. */
  focusInComponent(view: SomeView | ViewComponent, anchor?: FocusAnchor | NodeReferenceData): boolean {
    log.trace("canvas.focusInComponent", view, anchor);

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
      throw new Error(`no component for view ${viewPtr != null ? describeNode(viewPtr) : getVueComponentType(view)}`);
    }

    // if no anchor is given, try to use existing focus state
    if (anchor == null && (viewData?.focus?.nodesPtr?.length ?? 0) > 0) {
      const child = this.getViewData(viewData!.focus!.nodesPtr[0]);
      if (child != null) {
        // if we have a focus state we must use it, even if it didn't actually focus in the component
        //  (so we 'emulate' the focus in the component by calling onComponentFocused directly)
        if (!this.focusInComponent(child)) this.onComponentFocused(component);
        return true;
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
    log.trace("canvas.restoreComponentFocus", this.spacePtr.value);
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
      // NOTE: we enforce that el must be a single element for all Views with a lint rule
      if ((instance as any).vnode.el == null) console.warn("canvas.missingEl", getVueComponentType(instance), instance);
      else (instance as any).vnode.el.__viewComponent = instance;
    });

    // register
    // NOTE @Cleanup: 'self'/'id' should never change, so no need to watch in Canvas.registerView?
    let oldComponentId: string | null = null;
    watch(
      () => self.value?.id ?? id?.value ?? null,
      () => {
        if (oldComponentId != null && this.viewRefsById.value[oldComponentId] === instance)
          delete this.viewRefsById.value[oldComponentId];
        const componentId = self.value?.id ?? id?.value!;
        const existingComponent = this.viewRefsById.value[componentId];
        if (existingComponent != null) {
          // NOTE: checking for duplicate components only works reliably on next tick
          //  because we may be registering a new component before the old component is unmounted
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
      if (ROOT_VIEW_TYPES.includes(view.type)) return view;
      view = this.graph.get(view.parentPtr) as ViewData;
    }
    return null; // not found
  }

  /** Gets all the open frames (direct children of Window views, not reactive) */
  get currentFrames(): ViewData[] {
    if (this.spacePtr.value == null) return [];
    const getFrames = (view: ViewData): ViewData[] => {
      if (view.type == ViewType.WINDOW) {
        return this.graph.getChildren(view, NodeType.VIEW).flatMap(getFrames);
      } else {
        return [view];
      }
    };
    const frames = this.graph.getChildren(this.spacePtr.value, NodeType.VIEW).flatMap(getFrames);
    return frames;
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
      let primary = this.focusedRoot ?? this.currentFrames[0];
      if (primary == null) {
        // no root, reset space
        log.info("canvas.repairCanvas", this.spacePtr.value);
        const space = this.graph.getOrFail(this.spacePtr.value!);
        primary = setupEmptyCanvas(tx, space).primary;
      }

      // create & focus
      const rootChildren = this.graph.getChildren(primary, NodeType.VIEW);
      const newView = makeNode({
        ...view,
        metatype: NodeType.VIEW,
        packagePtr: primary.packagePtr,
        orderKey: generateKeyBetween(rootChildren[-1]?.orderKey ?? null, null),
        parentPtr: toNodeReference(primary),
        icon: toIconMaybe(view.icon),
      });
      tx.create(newView);
      this.focus(tx, { view: newView });
    } else if (options?.ifPresent == "focus") {
      this.focus(tx, { view: existing });
    } else if (options?.ifPresent == "upsertAndFocus") {
      tx.update(existing, { icon: toIconMaybe(view.icon) });
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
    log.debug("canvas.remove", view);
    const parent = graph.get(view.parentPtr!) as ViewData;
    tx.softDelete(view);
    this.cleanupRootViews(tx, graph, parent);
  }

  /**
   * Cleanup previously split (sub-)root views that are no longer needed.
   * NOTE: righ tnow we only close sub root views because it's annoying to have your layout change because you accidentally closed a tab.
   *  (and the 'layout' is usually your root splits)
   */
  cleanupRootViews(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
    if (!ROOT_VIEW_TYPES.includes(view.type)) return;
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
    if (split?.metatype != BenchType.VIEW)
      throw new Error(
        `no enclosing split view: [parent=${BenchType[parent.metatype]}, parent.type=${ViewType[parent.type]}]`,
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
        name: "Split",
        orientation,
      });
      tx.create(split);
      tx.move({ ...parent, parentPtr: toNodeReference(split) });
      tx.update(parent, { size: undefined, orderKey: isOrderFlipped ? "a0" : "a1" });

      // and a new tab wrapper
      const viewParent = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
        parentPtr: toNodeReference(split),
        packagePtr: parent.packagePtr,
        orderKey: isOrderFlipped ? "a1" : "a0",
      });
      tx.create(viewParent);
      tx.move({ ...child, parentPtr: toNodeReference(viewParent) });
      tx.update(child, { size: undefined, orderKey: "a0" });
    } else {
      // 'split' size between self and child with a new tab wrapper
      const halfSize = splitBox(parent.size!);
      const newSplitParent = makeNode({
        metatype: NodeType.VIEW,
        type: ViewType.TAB,
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
      tx.update(child, { size: halfSize, orderKey: "a0" });
      tx.update(parent, { size: halfSize });
    }
    this.cleanupRootViews(tx, graph, graph.get(child.parentPtr!) as ViewData);
  }
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
export function clearCanvas(tx: Transaction, graph: ReadNodeGraph, space: SpaceData) {
  const roots = graph.getChildren(space, NodeType.VIEW);
  for (const root of roots) {
    tx.softDelete(root);
  }
}

/** Sets up a minimal empty space with one root tab */
export function setupEmptyCanvas(tx: Transaction, space: SpaceData): { primary: ViewData } {
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

/** Setups up the default three-frame canvas (side, primary, secondary views) */
export function setupDefaultCanvas(
  tx: Transaction,
  space: SpaceData,
): { side: ViewData; primary: ViewData; secondary: ViewData } {
  const window = makeMainWindow(space, tx);
  // root splits
  const side = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.SPLIT,
    parentPtr: toNodeReference(window),
    packagePtr: space.packagePtr,
    orientation: Orientation.VERTICAL,
    orderKey: "a0",
    name: "Side",
    title: "Side",
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
  const secondary = tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.TAB,
    parentPtr: toNodeReference(window),
    packagePtr: space.packagePtr,
    orderKey: "a2",
    name: "Secondary",
    title: "Secondary",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1000 }),
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
    type: ViewType.EXPLORER,
    parentPtr: toNodeReference(sideTop),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Explorer",
    title: "Explorer",
  });
  tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.OUTLINE,
    parentPtr: toNodeReference(sideBottom),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Outline",
    title: "Outline",
  });

  // primary
  // ...?

  // secondary
  tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.INSPECTOR,
    parentPtr: toNodeReference(secondary),
    packagePtr: space.packagePtr,
    orderKey: "a0",
    name: "Inspector",
    title: "Inspector",
  });
  tx.create({
    metatype: NodeType.VIEW,
    type: ViewType.LIBRARY,
    parentPtr: toNodeReference(secondary),
    packagePtr: space.packagePtr,
    orderKey: "a1",
    name: "Library",
    title: "Library",
  });

  return { side, primary, secondary };
}
