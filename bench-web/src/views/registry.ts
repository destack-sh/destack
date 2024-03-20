import { BenchType, NodeReferenceData, NodeType, SelectionKind, SpaceData, ViewData } from "@/proto/wire";
import { toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { NodeKey, ReadNodeGraph } from "@/system/graph";
import type { Transaction, TransactionBuffer } from "@/system/transaction";
import { log } from "@/utils/log";
import type { ViewComponent } from "@/views";
import type { FocusAnchor } from "@/views/common";
import type { ViewModel } from "@sentry/vue/types/types";
import { useActiveElement, useEventListener } from "@vueuse/core";
import {
  computed,
  shallowRef,
  type ComponentInstance,
  type Ref,
  watch,
  getCurrentInstance,
  onBeforeUnmount,
  triggerRef,
  nextTick,
} from "vue";

/** Finds the closest Vue component */
export function findVueComponent(el: HTMLElement): ComponentInstance<any> | null {
  while (el != null) {
    if ((el as any).__vueParentComponent != null) return (el as any).__vueParentComponent;
    else el = el.parentElement!;
  }
  return null;
}

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

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds any closest ViewComponent ancestor. */
export function findViewComponent(e: HTMLElement | ComponentInstance<any>): ViewComponent | null {
  let vueComponent = e instanceof HTMLElement ? findVueComponent(e) : e;
  while (vueComponent != null) {
    if (isViewComponent(vueComponent)) return vueComponent;
    vueComponent = vueComponent.parent;
  }
  return null;
}

/** Finds any closest identified (not anonymous) ViewComponent ancestor */
export function findIdentifiedViewComponent(e: HTMLElement | ComponentInstance<any>): ViewComponent | null {
  let vueComponent = e instanceof HTMLElement ? findVueComponent(e) : e;
  while (vueComponent != null) {
    if (isIdentifiedViewComponent(vueComponent)) return vueComponent;
    vueComponent = vueComponent.parent;
  }
  return null;
}

/** Collect all view components from the given component upwards (inclusive) */
export function collectViewComponentsUp(componentOrEl: ComponentInstance<any> | HTMLElement): ViewComponent[] {
  let component = componentOrEl instanceof HTMLElement ? findVueComponent(componentOrEl) : componentOrEl;
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
        const component = (child as any).__vueParentComponent as ComponentInstance<any> | null;
        if (component != null && component !== instance) components.push(component);
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

type SomeView = NodeReferenceData | ViewData;

/**
 * A registry for linking Views, their Vue components, and their HTML elements.
 * Some of our View components may not have an associated View, so we track them with a derived id.
 * NOTE: ViewRegistry is effectively a global singleton (currently).
 */
export class ViewRegistry {
  spacePtr: Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction; // for when we're not given a transaction to work with (e.g. browser events)
  private viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});

  // absolutely focused views/components (focused from the top down)
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up
  focusedView: Ref<NodeReferenceData | null> = shallowRef(null);

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
    // and any other element
    useEventListener("mousedown", (e) => {
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
      this.focusedView.value = null;
    } else if (this.focusedViewComponent.value !== component) {
      this.focusedViewComponent.value = component;
      const componentsById: Record<string, ViewComponent> = {};
      collectViewComponentsUp(component).forEach((c) => {
        componentsById[getViewComponentId(c)] = c;
      });
      this.focusedViewComponentsById.value = componentsById;
      this.focusedView.value = findIdentifiedViewComponent(component)?.exposed.self?.value ?? null;
    }

    // update graph focus state
    if (this.focusedView.value != null && wasDifferent) {
      this.focusInGraph(this.txFactory(), { view: this.focusedView.value });
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
    focus: { view: SomeView; parent?: SomeView; anchor?: FocusAnchor; hasBrowserFocus?: boolean },
  ) {
    log.debug("view.focus", focus);
    this.focusInGraph(tx, focus);
    if (!focus.hasBrowserFocus) nextTick(() => this.focusInComponent(focus.view, focus.anchor));
  }

  /** Focuses the given view absolustely in the graph. */
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
    log.trace("view.componentFocus", view, anchor);

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
    if (component == null) throw new Error(`no component for view: ${(viewData ?? view)?.id}`);

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
        } else if (isViewComponent(focusResult)) {
          // an inner view component to focus
          this.focusInComponent(focusResult);
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
    log.debug("view.restoreFocus", this.spacePtr.value);
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

  /** Registers the current Vue instance as the given identity */
  registerCurrent(self: Ref<NodeReferenceData | undefined>, id?: Ref<string>): ViewComponent {
    const instance = getCurrentInstance() as ViewComponent | null;
    if (instance == null) throw new Error("no current Vue instance");
    let oldComponentId: string | null = null;
    // register
    watch(
      [() => self.value?.id, () => id?.value],
      () => {
        if (oldComponentId != null) delete this.viewRefsById.value[oldComponentId];
        const componentId = self.value?.id ?? id?.value!;
        if (this.viewRefsById.value[componentId] != null) {
          // This only happens for our own components, so we should fail hard.
          //  (This is almost certainly a name-derived id from makeViewId, so we re-used an anonymous views' name accidentally).
          throw new Error(`duplicate component id: ${componentId}`);
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
      if (oldComponentId != null) {
        delete this.viewRefsById.value[oldComponentId];
        triggerRef(this.viewRefsById);
      }
    });
    return instance;
  }
}
