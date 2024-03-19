import { ViewData, NodeType, NodeReferenceData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ReadNodeGraph, NodeKey } from "@/system/graph";
import { activeElement } from "@/system/space";
import type { ViewComponent } from "@/views";
import { computed, shallowRef, type ComponentInstance, type Ref } from "vue";

/** Finds the closest Vue component */
function findVueComponent(el: HTMLElement): ComponentInstance<any> | null {
  while (el != null) {
    if ((el as any).__vueParentComponent != null) return (el as any).__vueParentComponent;
    el = el.parentElement!;
  }
  return null;
}

/**
 * A registry for linking Views, their Vue components, and their HTML elements.
 */
export class ViewRegistry {
  graph: ReadNodeGraph;
  viewRefsById: Ref<Record<string, ViewComponent>> = shallowRef({});
  focusedView: Ref<ViewData | null>; // nocheckin: track focus even if activeElement is elsewhere (e.g. omnibar)
  focusedViews: Ref<ViewData[]>;

  constructor(graph: ReadNodeGraph) {
    this.graph = graph;

    this.focusedView = computed(() => {
      const focused = activeElement.value;
      if (focused == null) return null;
      const viewPtr = this.findViewPtr(focused);
      if (viewPtr == null) return null;
      else return this.graph.get(viewPtr) as ViewData;
    });
    this.focusedViews = computed(() => {
      const focused = this.focusedView.value;
      if (focused == null) return [];
      const focusedView = this.graph.get(focused as NodeKey<any>) as ViewData;
      return this.graph.getAncestors(focusedView, [NodeType.VIEW]) as ViewData[];
    });
  }

  /** Finds the closest ViewComponent ancestor. */
  findViewComponent(e: HTMLElement): ViewComponent | null {
    // first find the Vue component
    let vueComponent = findVueComponent(e);
    // then look for View component
    while (vueComponent != null) {
      if (vueComponent.exposed?.self?.value != null) {
        return vueComponent as ViewComponent;
      }
      vueComponent = vueComponent.parent;
    }
    return null;
  }

  /** Finds the View pointer of the closest ViewComponent ancestor. */
  findViewPtr(e: HTMLElement): TypedNodeReferenceData<NodeType.VIEW> | null {
    const component = this.findViewComponent(e);
    return (component?.exposed.self?.value ?? null) as TypedNodeReferenceData<NodeType.VIEW> | null;
  }

  /** Whether the given view is directly focused */
  isFocused(node: NodeReferenceData): boolean {
    return this.focusedView.value?.id == node.id;
  }

  /** Whether the given view is directly focused (reactive) */
  isFocusedRef(node: Ref<NodeReferenceData> | null): Ref<boolean> {
    return computed(() => node?.value != null && this.isFocused(node.value));
  }

  /** Whether anything inside the given view is focused */
  isFocusedWithin(node: NodeReferenceData): boolean {
    return this.focusedViews.value.some((v) => v.id == node.id);
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
