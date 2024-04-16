import type { IconData } from "@/proto/wire";
import {
  fireAction,
  getAction,
  getActionsLike,
  isActionImplemented,
  type Action,
  type ActionBuiltinId,
  type ActionContext,
  type ActionFilter,
} from "@/system/action";
import { canvas } from "@/system/space";
import { type FloatingOptions } from "@/utils/floating";
import { pretendReadonly } from "@/utils/ref";
import type { ViewComponent } from "@/views";
import { collectViewComponentsUp } from "@/views/canvas";
import type { MaybeElement } from "@vueuse/core";
import { computed, shallowRef, toValue, type Directive, type Ref } from "vue";

export type MenuContext = ActionContext & {
  triggerElement?: MaybeElement;
};

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items: MenuItem[];
  context?: MenuContext;
};

export const MENU_ITEM_TYPES = ["generic", "option", "toggle"] as const;
export type MenuItemType = (typeof MENU_ITEM_TYPES)[number];

export type MenuItem = {
  id: string;
  type: MenuItemType;
  icon?: string | IconData;
  title: string;
  category?: string;
  shortcuts?: string[];
  isDisabled?: boolean;
  isLoading?: boolean;
  isChecked?: boolean;
  action: ((menu: MenuInfo) => void) | MenuInfo;
};

/** Gets the view context for a given menu (item) */
function getMenuContextViews(context?: MenuContext): ViewComponent[] {
  if (context?.triggerElement) {
    return collectViewComponentsUp(context.triggerElement);
  } else {
    return canvas.focusedViewComponents;
  }
}

/** Maps an action to a typical menu item in context  */
export function menuItemFromAction(
  actionOrId: Action | ActionBuiltinId,
  override?: Partial<MenuItem> & { context?: MenuContext; contextViews?: ViewComponent[] },
): MenuItem {
  const action = typeof actionOrId === "string" ? getAction(actionOrId) : actionOrId;

  // figure out whether the action is available in this context
  const contextViews = override?.contextViews ?? getMenuContextViews(override?.context);
  const isDisabled = !isActionImplemented(action, contextViews);

  // map to action
  return {
    id: action.id,
    type: MENU_ITEM_TYPES.includes(action.type as any) ? (action.type as MenuItemType) : "generic",
    icon: action.icon?.faName,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled,
    isChecked: action.type == "toggle" && action.isChecked?.value,
    // default to subcategory since we usually group menus by category(ish)
    category: action.subcategory ?? action.category,
    action: (menu: MenuInfo) => {
      const contextViews = getMenuContextViews(menu.context);
      const actionContext = { ...menu.context, ...(override?.context ?? {}) };
      fireAction(action, contextViews, actionContext);
    },
    ...override,
  };
}

/** Convenience wrapper around action filter & menu item mapping */
export function menuActionsLike(
  filter: ActionFilter,
  override?: Partial<MenuItem> & { context?: MenuContext },
): MenuItem[] {
  const contextViews = getMenuContextViews(override?.context);
  const actions = getActionsLike(filter).map((action) =>
    menuItemFromAction(action, { contextViews, ...(override ?? {}) }),
  );
  return actions;
}

//
// Context menus
//

export const OVERLAY_MENU_DEFAULT_FLOATING_OPTIONS: FloatingOptions = {
  placement: "bottom-right",
  referenceMargin: 0,
  containerMargin: 8,
};

type ComponentMenuInfo<T extends { props: object }> = {
  kind: "component";
  component: T;
  props: T["props"];
  context?: MenuContext;
};
export type OverlayMenuInfo = (({ kind: "menu" } & MenuInfo) | ComponentMenuInfo<any>) & {
  onApply?(value?: any): void;
  onClose?(): void;
} & FloatingOptions;

/** The triggering element with some extra state */
type OverlayMenuTriggerElement = HTMLElement & {
  contextMenuOnEvent?: (e: MouseEvent) => void;
};

type OverlayMenuInstance = {
  id: number;
  info: OverlayMenuInfo;
  trigger: HTMLElement;
  reference: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement;
};

const _activeOverlayMenu: Ref<OverlayMenuInstance | null> = shallowRef(null);
export const activeOverlayMenu = pretendReadonly(_activeOverlayMenu);
export const hasActiveOverlayMenu = computed(() => _activeOverlayMenu.value != null);

let overlayMenuId = 0;
function newOverlayMenuId() {
  return overlayMenuId++;
}

export function createOverlayMenu(
  trigger: HTMLElement,
  reference: { x: number; y: number } | HTMLElement | SVGElement,
  container: HTMLElement | SVGElement | undefined,
  info: OverlayMenuInfo | ((ctx: MenuContext) => OverlayMenuInfo),
): OverlayMenuInstance {
  const triggerNode = canvas.findViewData(trigger) ?? undefined;
  const context: MenuContext = { triggerElement: trigger, triggerNode };
  info = {
    ...OVERLAY_MENU_DEFAULT_FLOATING_OPTIONS,
    ...(typeof info === "function" ? info(context) : info),
    context,
  };
  if (info.kind != "component" && info.kind != "menu")
    throw new Error(`invalid overlay menu kind: ${(info as any).kind}`);

  const instance = { id: newOverlayMenuId(), info, trigger, reference, container };
  _activeOverlayMenu.value = instance;
  trigger.dataset.contextmenu = "true";
  return instance;
}

export function destroyOverlayMenu(instance?: OverlayMenuInstance) {
  if (instance == null || _activeOverlayMenu.value === instance) {
    if (activeOverlayMenu.value != null) {
      _activeOverlayMenu.value!.trigger.dataset.contextmenu = undefined;
    }
    _activeOverlayMenu.value = null;
  }
}

function makeOverlayMenuDirective(options: {
  event: "contextmenu" | "click";
  reference: "trigger" | "self";
}): Directive<MaybeElement, OverlayMenuInfo> {
  return {
    mounted(el, binding) {
      const triggerEl = el as OverlayMenuTriggerElement;
      triggerEl.contextMenuOnEvent = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        const reference = options.reference == "self" ? triggerEl : { x: e.clientX, y: e.clientY };
        createOverlayMenu(triggerEl, reference, undefined, binding.value);
      };
      triggerEl.addEventListener(options.event, triggerEl.contextMenuOnEvent);
    },

    unmounted(el) {
      const triggerEl = el as OverlayMenuTriggerElement;
      if (triggerEl.contextMenuOnEvent) triggerEl.removeEventListener(options.event, triggerEl.contextMenuOnEvent);
    },
  };
}

export const CONTEXT_MENU_DIRECTIVE = makeOverlayMenuDirective({ event: "contextmenu", reference: "trigger" });
export const MENU_DIRECTIVE = makeOverlayMenuDirective({ event: "click", reference: "self" });
