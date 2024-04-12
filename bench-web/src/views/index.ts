import { BoxData, NodeReferenceData, ViewData, ViewType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import type { ViewExposed } from "@/views/common";
import { v4 } from "uuid";
import { computed, getCurrentInstance, type ComponentInstance, type Ref } from "vue";

export type ViewComponent = {
  new (): ComponentInstance<any>;
  props: { self: NodeReferenceData };
  exposed: ViewExposed;
};

// TODO :Architecture: how to register, type & wrap Views/Components?
//  This setup is kind of annoying because it forces a complete reload during development.
const COMPONENT_BY_VIEW_TYPE_LAZY = {
  // kernel
  [ViewType.USER_WIZARD]: import("@/views/kernel/UserWizard.vue"),
  [ViewType.BENCH_WIZARD]: import("@/views/kernel/BenchWizard.vue"),
  // system
  [ViewType.PAGE]: import("@/views/system/Page.vue"),
  [ViewType.BLOCK]: import("@/views/system/Block.vue"),
  [ViewType.MOCK]: import("@/views/system/Mock.vue"),
  [ViewType.EXPLORE]: import("@/views/system/Explore.vue"),
  [ViewType.OUTLINE]: import("@/views/system/Explore.vue"), // shared with Explore
  [ViewType.INSPECT]: import("@/views/system/Inspect.vue"),
  [ViewType.CREATE]: import("@/views/system/Create.vue"),
  // containers
  [ViewType.WINDOW]: import("@/views/containers/Split.vue"), // shared with Split
  [ViewType.TAB]: import("@/views/containers/Tab.vue"),
  [ViewType.SPLIT]: import("@/views/containers/Split.vue"),
  [ViewType.GROUP]: import("@/views/containers/Group.vue"),
  // controls
  [ViewType.BUTTON]: import("@/views/controls/Button.vue"),
  // content
  [ViewType.PLAIN_TEXT]: import("@/views/content/PlainText.vue"),
};
const COMPONENT_BY_VIEW_TYPE = {} as Record<ViewType, ViewComponent>;
let didRegisterComponents = false;
export async function registerViewComponents() {
  if (didRegisterComponents) throw new Error("components already registered");
  for (const [viewType, component] of Object.entries(COMPONENT_BY_VIEW_TYPE_LAZY)) {
    COMPONENT_BY_VIEW_TYPE[viewType as unknown as ViewType] = (await component).default as unknown as ViewComponent;
  }
  didRegisterComponents = true;
}

export function getViewComponent<T extends ViewType>(viewType: T): (typeof COMPONENT_BY_VIEW_TYPE)[T] | null {
  if (!didRegisterComponents) throw new Error("Components not registered");
  const component = COMPONENT_BY_VIEW_TYPE[viewType];
  return component ?? null;
}

export function getViewBinding(view: ViewData, size: Omit<BoxData, "metatype">): Record<string, any> {
  // probably need to filter these?
  const component = COMPONENT_BY_VIEW_TYPE[view.type];
  if (!component) throw new Error(`no component for view type: ${view.type}`);
  const filteredProps = {};
  for (const propName in component.props) {
    if (propName == "size") (filteredProps as any)[propName] = size;
    else if (propName == "self") (filteredProps as any)[propName] = toNodeReference(view);
    else if (propName == "id") (filteredProps as any)[propName] = view.id;
    else if (propName in view) (filteredProps as any)[propName] = (view as any)[propName];
  }
  return filteredProps;
}

/** Creates an id for an 'anonymous' view */
function deriveViewId(selfId: string, name: string): string {
  return `${selfId}.${name}`;
}

/** Creates a dynamic view id ref for View components that sometimes don't have a 'self' node identity */
export function makeViewId(props: { self?: NodeReferenceData; name?: string | null }): Ref<string> {
  const instance = getCurrentInstance()!;
  if (instance == null) throw new Error("no Vue instance");
  return computed(() => {
    if (props.self?.id != null) return props.self.id;

    const instanceInternalId = instance.uid;
    let parent = instance.parent;
    while (parent != null) {
      const exposed = (parent as unknown as ViewComponent).exposed;
      if (exposed?.self?.value?.id != null)
        return deriveViewId(exposed.self.value.id, props.name ?? instanceInternalId.toString());
      else if (exposed?.id?.value != null)
        return deriveViewId(exposed.id.value, props.name ?? instanceInternalId.toString());
      parent = parent.parent;
    }
    // this is a component outside of parent view, just use a random id
    return v4();
  });
}
