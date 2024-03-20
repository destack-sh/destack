import { BenchType, NodeReferenceData, NodeType, SelectionKind, SpaceData, ViewData } from "@/proto/wire";
import { toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { NodeKey, ReadNodeGraph } from "@/system/graph";
import type { Transaction } from "@/system/transaction";
import { log } from "@/utils/log";
import type { ViewComponent } from "@/views";
import { useActiveElement } from "@vueuse/core";
import {
  computed,
  shallowRef,
  type ComponentInstance,
  type Ref,
  watch,
  getCurrentInstance,
  onBeforeUnmount,
  triggerRef,
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

export function isViewComponent(component: ComponentInstance<any>): component is ViewComponent {
  return (component as any).exposed?.self != null || (component as any).exposed?.id != null;
}

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds the closest ViewComponent ancestor. */
export function findViewComponent(e: HTMLElement): ViewComponent | null {
  // first find the Vue component
  let vueComponent = findVueComponent(e);
  // then look for View component
  while (vueComponent != null) {
    if (isViewComponent(vueComponent)) return vueComponent;
    vueComponent = vueComponent.parent;
  }
  return null;
}

/** Collect all view components from the given component upwards (inclusive) */
export function collectViewComponents(componentOrEl: ComponentInstance<any> | HTMLElement): ViewComponent[] {
  let component = componentOrEl instanceof HTMLElement ? findVueComponent(componentOrEl) : componentOrEl;
  const components = [];
  while (component != null) {
    if (isViewComponent(component)) components.push(component);
    component = component.parent;
  }
  return components;
}

const activeElement = useActiveElement();

/** Traverses the DOM up to check if any element is marked as outside view */
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
 */
export class ViewRegistry {
  graph: ReadNodeGraph;
  private viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up
  focusedView: Ref<NodeReferenceData | null> = shallowRef(null);

  constructor(graph: ReadNodeGraph) {
    this.graph = graph;

    // update focus manually when active element changes (if not in excluded elements)
    watch(activeElement, () => {
      if (activeElement.value == null || isOutsideView(activeElement.value!)) return;

      const viewComponent = findViewComponent(activeElement.value);
      if (viewComponent == null) {
        this.focusedViewComponent.value = null;
        this.focusedViewComponentsById.value = {};
        this.focusedView.value = null;
      } else if (this.focusedViewComponent.value !== viewComponent) {
        this.focusedViewComponent.value = viewComponent;
        const componentsById: Record<string, ViewComponent> = {};
        collectViewComponents(viewComponent).forEach((c) => {
          componentsById[getViewComponentId(c)] = c;
        });
        this.focusedViewComponentsById.value = componentsById;
        this.focusedView.value = viewComponent.exposed.self?.value ?? null;
      }
    });
  }

  get focusedViewComponents(): ViewComponent[] {
    return Object.values(this.focusedViewComponentsById.value);
  }

  /** Gets the view component for a certain view identity (self.id or anonymous id) */
  getViewComponent<T extends ViewComponent>(id: string): T | null {
    return this.viewRefsById.value[id] as T | null;
  }

  /** Resolve the view data */
  getViewData(view: SomeView): ViewData {
    if (view.metatype == BenchType.VIEW) return view as ViewData;
    else return this.graph.get(view as NodeKey<NodeType.VIEW>) as ViewData;
  }

  /** Focus the given views (and all ancestor views) */
  focus(tx: Transaction, self: SomeView, focus: { view: SomeView }) {
    log.debug("focus", self, focus);

    // focus view and all ancestors
    let parent: ViewData | SpaceData | null = this.getViewData(self);
    let child = this.getViewData(focus.view);
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
  }

  /** Whether the given view is directly focused (not an ancestor/descendant view) */
  isFocusedDirectly(view: SomeView): boolean {
    return this.focusedView.value?.id == view.id;
  }

  // type FocusInfo = { isAbsolute: boolean, kind: "self" | "ancestor" | "descendant" | null };

  /** Finds the View identity of the closest ViewComponent ancestor. */
  findViewPtr(e: HTMLElement): TypedNodeReferenceData<NodeType.VIEW> | null {
    const component = findViewComponent(e);
    return (component?.exposed.self?.value ?? null) as TypedNodeReferenceData<NodeType.VIEW> | null;
  }

  /** Registers the current Vue instance as the given identity */
  registerCurrent(self: Ref<NodeReferenceData | undefined>, id?: Ref<string>) {
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
  }
}
