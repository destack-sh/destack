import { BoxData, NodeReferenceData, ViewData, ViewType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { type ComponentInstance } from "vue";

export type ViewComponent = {
  new (): ComponentInstance<any>;
  props: { self: NodeReferenceData };
  exposed: { self?: NodeReferenceData };
} 

// TODO :Architecture: how to register, type & wrap Views/Components?
//  This setup is kind of annoying because it forces a complete reload during development.
const COMPONENT_BY_VIEW_TYPE_LAZY = {
  // kernel
  [ViewType.USER_WIZARD]: import("@/views/kernel/UserWizard.vue"),
  [ViewType.BENCH_WIZARD]: import("@/views/kernel/BenchWizard.vue"),
  // system
  [ViewType.PAGE]: import("@/views/system/Page.vue"),
  // containers
  [ViewType.WINDOWED]: import("@/views/containers/Windowed.vue"),
  [ViewType.TABBED]: import("@/views/containers/Tabbed.vue"),
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
    else if (propName in view) (filteredProps as any)[propName] = (view as any)[propName];
  }
  return filteredProps;
}
