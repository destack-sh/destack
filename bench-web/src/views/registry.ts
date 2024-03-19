import { NodeReferenceData, NodeType } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
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

/**
 * A registry for linking Views, their Vue components, and their HTML elements.
 * Some of our View components may not have an associated View, so we track them with a derived id.
 */
export class ViewRegistry {
  graph: ReadNodeGraph;
  viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});
  focusedViewComponent: Ref<ViewComponent | null> = shallowRef(null);
  focusedViewComponentsById: Ref<Record<string, ViewComponent>> = shallowRef({}); // order is bottom up

  constructor(graph: ReadNodeGraph) {
    this.graph = graph;

    watch(activeElement, () => {
      if (activeElement.value == null) {
        this.focusedViewComponent.value = null;
        this.focusedViewComponentsById.value = {};
      } else {
        const viewComponent = findViewComponent(activeElement.value);
        if (viewComponent == null) {
          this.focusedViewComponent.value = null;
          this.focusedViewComponentsById.value = {};
        } else if (this.focusedViewComponent.value !== viewComponent) {
          this.focusedViewComponent.value = viewComponent;
          const componentsById: Record<string, ViewComponent> = {};
          collectViewComponents(viewComponent).forEach((c) => {
            componentsById[getViewComponentId(c)] = c;
          });
          this.focusedViewComponentsById.value = componentsById;
        }
      }
    });
  }

  get focusedViewComponents(): ViewComponent[] {
    return Object.values(this.focusedViewComponentsById.value);
  }

  /** Finds the View pointer of the closest ViewComponent ancestor. */
  findViewPtr(e: HTMLElement): TypedNodeReferenceData<NodeType.VIEW> | null {
    const component = findViewComponent(e);
    return (component?.exposed.self?.value ?? null) as TypedNodeReferenceData<NodeType.VIEW> | null;
  }

  /** Whether the given view is directly focused */
  isFocused(node: NodeReferenceData): boolean {
    return this.focusedViewComponent.value?.exposed?.id == node.id;
  }

  /** Whether the given view is directly focused (reactive) */
  isFocusedRef(node: Ref<NodeReferenceData> | null): Ref<boolean> {
    return computed(() => node?.value != null && this.isFocused(node.value));
  }

  /** Whether anything inside the given view is focused */
  isFocusedWithin(node: NodeReferenceData): boolean {
    return this.focusedViewComponentsById.value[node.id ?? ""] != null;
  }

  /** Whether anything inside the given view is focused (reactive) */
  isFocusedWithinRef(node: Ref<NodeReferenceData> | null): Ref<boolean> {
    return computed(() => node?.value != null && this.isFocusedWithin(node.value));
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
        if (this.viewRefsById.value[componentId] != null)
        // This only happens for our own components, so we should fail hard.
        //  (This must be a name-derived id from makeViewId, with colliding names).  
          throw new Error(`duplicate component id: ${componentId}`); 
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
