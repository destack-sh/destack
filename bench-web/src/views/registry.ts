import { NodeReferenceData, NodeType } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import { log } from "@/utils/log";
import type { ViewComponent } from "@/views";
import { useActiveElement } from "@vueuse/core";
import { computed, shallowRef, type ComponentInstance, type Ref, watch } from "vue";

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

  /** Register/unregister the given view's component instance */
  register(viewSelf: NodeReferenceData, instance: ViewComponent | undefined) {
    if (viewSelf.id == null) throw new Error(`node has no id: ${viewSelf}`);
    if (instance == null) {
      delete this.viewRefsById.value[viewSelf.id];
    } else {
      this.viewRefsById.value[viewSelf.id] = instance;
    }
  }
}
