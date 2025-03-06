import { RectangleData, ViewData, ViewType } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { getVueComponentType } from "@/ui/view";
import type { ViewComponent } from "@/views/common";

// NOTE!: Only put ABSOLUTELY NECESSARY definitions for the view registry here.
// Any change in any component or import will cause all dependencies to hot reload during development!
// NOTE: Architecture: we register these components lazily to avoid force reloading everything during local development.
//  (Otherwise any change in any component requires this file to be reloaded, forcing *all* components to be reloaded.)
// NOTE: sync with inverse registry in common :ViewRegistry
//  (see above for why we can't import from here)
const COMPONENT_BY_VIEW_TYPE_LAZY = {
  //
  // Intrinsics
  //

  // system
  // nodes
  [ViewType.PAGE]: () => import("@/views/nodes/Page.vue"),
  [ViewType.BLOCK]: () => import("@/views/nodes/Block.vue"),
  [ViewType.FIELD]: () => import("@/views/nodes/Field.vue"),
  [ViewType.ACTION]: () => import("@/views/nodes/Action.vue"),
  [ViewType.LINK]: () => import("@/views/nodes/Link.vue"),
  [ViewType.RUN]: () => import("@/views/nodes/Run.vue"),
  [ViewType.DATABASE]: () => import("@/views/nodes/Database.vue"),
  [ViewType.FLOW]: () => import("@/views/nodes/Flow.vue"),
  [ViewType.KIT]: () => import("@/views/nodes/Kit.vue"),
  [ViewType.THREAD]: () => import("@/views/nodes/Thread.vue"),
  [ViewType.CHANNEL]: () => import("@/views/nodes/Thread.vue"), // shared with Thread
  [ViewType.TASK]: () => import("@/views/nodes/Task.vue"),
  // structs
  [ViewType.TYPE]: () => import("@/views/objects/Type.vue"),
  [ViewType.OBJECT]: () => import("@/views/objects/Object.vue"),
  [ViewType.FIELD_LIST]: () => import("@/views/objects/FieldList.vue"),
  [ViewType.COMPUTED_VALUE]: () => import("@/views/objects/ComputedValue.vue"),
  [ViewType.PATH]: () => import("@/views/objects/Path.vue"),
  // helpers
  [ViewType.USER_WIZARD]: () => import("@/views/helpers/UserWizard.vue"),
  [ViewType.BENCH_WIZARD]: () => import("@/views/helpers/BenchWizard.vue"),
  [ViewType.EMPTY]: () => import("@/views/helpers/Empty.vue"),
  [ViewType.SIDEBAR]: () => import("@/views/helpers/Sidebar.vue"),
  [ViewType.CONTEXT]: () => import("@/views/helpers/Context.vue"),
  [ViewType.CHAT]: () => import("@/views/helpers/Chat.vue"),
  [ViewType.CATALOG]: () => import("@/views/helpers/Catalog.vue"),
  [ViewType.ACTIVITY]: () => import("@/views/helpers/Activity.vue"),

  //
  // Organization
  //

  // layout
  [ViewType.WINDOW]: () => import("@/views/containers/Split.vue"), // shared with Split
  [ViewType.TAB]: () => import("@/views/containers/Tab.vue"),
  [ViewType.HISTORY]: () => import("@/views/containers/History.vue"),
  [ViewType.SPLIT]: () => import("@/views/containers/Split.vue"),
  [ViewType.SCROLL]: () => import("@/views/containers/Scroll.vue"),

  // groups

  // presentation

  // collections
  [ViewType.TREE]: () => import("@/views/collections/Tree.vue"),
  [ViewType.LIST]: () => import("@/views/collections/List.vue"),

  //
  // Style
  //

  // navigation

  // illustration

  // graphing

  //
  // Action
  //

  // controls
  [ViewType.BUTTON]: () => import("@/views/controls/Button.vue"),

  //
  // Content
  //

  // numeric
  [ViewType.NUMBER]: () => import("@/views/content/NativeInput.vue"),

  // stringy
  [ViewType.STRING]: () => import("@/views/content/NativeInput.vue"),
  [ViewType.TEXT]: () => import("@/views/content/Text.vue"),
  [ViewType.CODE]: () => import("@/views/content/Code.vue"),

  // selection
  [ViewType.TOGGLE]: () => import("@/views/content/Toggle.vue"),
  [ViewType.PICKER]: () => import("@/views/content/Picker.vue"),
  [ViewType.COLOR]: () => import("@/views/content/Color.vue"),
  [ViewType.ICON]: () => import("@/views/content/Icon.vue"),
  [ViewType.DATETIME]: () => import("@/views/content/Datetime.vue"),
  [ViewType.DURATION]: () => import("@/views/content/Duration.vue"),

  // file
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

export function getViewBinding(view: ViewData, size: Pick<RectangleData, "width" | "height">): Record<string, any> {
  // probably need to filter these?
  const component = COMPONENT_BY_VIEW_TYPE[view.type];
  if (!component) throw new Error(`no registered component for view type: ${view.type}`);
  const filteredProps = {};
  for (const propName in component.props) {
    if (propName == "size") (filteredProps as any)[propName] = size;
    else if (propName == "self") (filteredProps as any)[propName] = toNodeRef(view);
    else if (propName == "id") (filteredProps as any)[propName] = view.id;
    else if (propName in view) (filteredProps as any)[propName] = (view as any)[propName];
  }
  return filteredProps;
}
