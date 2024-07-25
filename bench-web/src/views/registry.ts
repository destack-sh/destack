import { BoxData, ViewData, ViewType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { getVueComponentType } from "@/views/canvas";
import type { ViewComponent } from "@/views/common";

// NOTE!: Only put ABSOLUTELY NECESSARY definitions for the view registry here.
// Any change in any component or import will cause all dependencies to hot reload!
// NOTE: Architecture: we register these components lazily to avoid force reloading everything during local development.
//  (Otherwise any change in any component requires this file to be reloaded, forcing *all* components to be reloaded.)
// NOTE: sync with inverse registry in common :ViewRegistry
//  (see above for why we can't import from here)
const COMPONENT_BY_VIEW_TYPE_LAZY = {
  // kernel
  [ViewType.USER_WIZARD]: () => import("@/views/kernel/UserWizard.vue"),
  [ViewType.BENCH_WIZARD]: () => import("@/views/kernel/BenchWizard.vue"),

  // system
  // nodes
  [ViewType.PAGE]: () => import("@/views/system/Page.vue"),
  [ViewType.BLOCK]: () => import("@/views/system/Block.vue"),
  [ViewType.FIELD]: () => import("@/views/system/Field.vue"),
  [ViewType.TYPE]: () => import("@/views/system/Type.vue"),
  [ViewType.OBJECT]: () => import("@/views/system/Object.vue"),
  // helpers
  [ViewType.EMPTY]: () => import("@/views/system/Empty.vue"),
  [ViewType.TREE]: () => import("@/views/system/Tree.vue"),
  [ViewType.INSPECT]: () => import("@/views/system/Inspect.vue"),
  [ViewType.CREATE]: () => import("@/views/system/Create.vue"),
  [ViewType.CHAT]: () => import("@/views/system/Chat.vue"),
  [ViewType.START]: () => import("@/views/system/Start.vue"),
  [ViewType.FEED]: () => import("@/views/system/Feed.vue"),

  // containers
  [ViewType.WINDOW]: () => import("@/views/containers/Split.vue"), // shared with Split
  [ViewType.TAB]: () => import("@/views/containers/Tab.vue"),
  [ViewType.SPLIT]: () => import("@/views/containers/Split.vue"),
  [ViewType.GROUP]: () => import("@/views/containers/Group.vue"),
  [ViewType.SCROLL]: () => import("@/views/containers/Scroll.vue"),

  // controls
  [ViewType.BUTTON]: () => import("@/views/controls/Button.vue"),

  // content
  [ViewType.VALUE]: () => import("@/views/content/Value.vue"),
  [ViewType.STRING]: () => import("@/views/content/NativeInput.vue"),
  [ViewType.NUMBER]: () => import("@/views/content/NativeInput.vue"),
  [ViewType.TEXT]: () => import("@/views/content/Text.vue"),
  [ViewType.CODE]: () => import("@/views/content/Code.vue"),
  [ViewType.TOGGLE]: () => import("@/views/content/Toggle.vue"),
  [ViewType.PICKER]: () => import("@/views/content/Picker.vue"),
  [ViewType.COLOR]: () => import("@/views/content/Color.vue"),
  [ViewType.ICON]: () => import("@/views/content/Icon.vue"),
  [ViewType.FILE]: () => import("@/views/content/File.vue"),
  [ViewType.IMAGE]: () => import("@/views/content/File.vue"), // shared with File
  [ViewType.AUDIO]: () => import("@/views/content/File.vue"), // shared with File
  [ViewType.VIDEO]: () => import("@/views/content/File.vue"), // shared with File
  [ViewType.DOCUMENT]: () => import("@/views/content/File.vue"), // shared with File
};
const COMPONENT_BY_VIEW_TYPE = {} as Record<ViewType, ViewComponent>;
const VIEW_TYPE_BY_COMPONENT_NAME = {} as Record<string, ViewType>;
let didRegisterComponents = false;
export async function registerViewComponents() {
  if (didRegisterComponents) throw new Error("components already registered");
  for (const [key, componentLazy] of Object.entries(COMPONENT_BY_VIEW_TYPE_LAZY)) {
    const component = (await componentLazy()).default as unknown as ViewComponent;
    const viewType = Number(key); // not sure why this is a string?
    if (isNaN(viewType)) throw new Error(`invalid view type: ${key}`);
    COMPONENT_BY_VIEW_TYPE[viewType as ViewType] = component;
    VIEW_TYPE_BY_COMPONENT_NAME[getVueComponentType(component)] = viewType as ViewType;
  }
  didRegisterComponents = true;
}

export function hasViewComponent(viewType: ViewType): boolean {
  return COMPONENT_BY_VIEW_TYPE[viewType] != null;
}

export function getViewComponent<T extends ViewType>(viewType: T): (typeof COMPONENT_BY_VIEW_TYPE)[T] | null {
  if (!didRegisterComponents) throw new Error("Components not registered");
  if (typeof viewType != "number") throw new Error(`invalid view type: ${viewType} (${typeof viewType})`);
  const component = COMPONENT_BY_VIEW_TYPE[viewType];
  return component ?? null;
}

export function getViewBinding(view: ViewData, size: Pick<BoxData, "width" | "height">): Record<string, any> {
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
