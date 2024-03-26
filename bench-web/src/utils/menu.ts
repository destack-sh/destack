import type { IconData } from "@/proto/wire";
import {
  fireAction,
  getAction,
  getActionsLike,
  isActionImplemented,
  type Action,
  type ActionBuiltinId,
  type ActionFilter,
} from "@/system/action";
import { canvas } from "@/system/space";
import { findFloatingContainer, type FloatingOptions } from "@/utils/floating";
import { pretendReadonly } from "@/utils/ref";
import type { ViewComponent } from "@/views";
import { collectViewComponentsUp } from "@/views/canvas";
import type { MaybeElement } from "@vueuse/core";
import { shallowRef, toValue, type Directive, type Ref } from "vue";

export type MenuContext = {
  triggerElement?: MaybeElement;
};

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items: MenuItem[];
  context?: MenuContext;
};

export type MenuItem = {
  id: string;
  icon?: string | IconData;
  title: string;
  category?: string;
  shortcuts?: string[];
  isDisabled?: boolean;
  isLoading?: boolean;
  action: ((menu: MenuInfo) => void) | MenuInfo;
};

export type MenuItemContextSource = "current" | "trigger-element";

/** Gets the view context for a given menu (item) */
export function getMenuContextViews(contextSource: MenuItemContextSource, context?: MenuContext): ViewComponent[] {
  if (contextSource == "current") {
    return canvas.focusedViewComponents;
  } else if (contextSource == "trigger-element") {
    if (context?.triggerElement == null) throw new Error("no trigger element in menu context");
    return collectViewComponentsUp(context.triggerElement);
  } else {
    throw new Error(`unexpected menu context: ${context}`);
  }
}

/** Maps an action to a typical menu item in context  */
export function menuItemFromAction(
  actionOrId: Action | ActionBuiltinId,
  override?: Partial<MenuItem> & { context?: MenuContext | MenuItemContextSource },
): MenuItem {
  const action = typeof actionOrId === "string" ? getAction(actionOrId) : actionOrId;

  // figure out whether the action is available in this context
  const context = override?.context ?? "current";
  const contextViews =
    typeof context == "string" ? getMenuContextViews(context) : getMenuContextViews("trigger-element", context);
  const isDisabled = !isActionImplemented(action, contextViews);

  // map to action
  return {
    id: action.id,
    icon: action.icon?.name,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled,
    // default to subcategory since we usually group menus by category(ish)
    category: action.subcategory ?? action.category,
    action: (menu: MenuInfo) => {
      const contextViews =
        context == "current" ? canvas.focusedViewComponents : getMenuContextViews("trigger-element", menu.context);
      fireAction(action, contextViews);
    },
    ...override,
  };
}

/** Convenience wrapper around action filter & menu item mapping */
export function menuActionsLike(
  filter: ActionFilter,
  override?: Partial<MenuItem> & { context?: MenuContext | MenuItemContextSource },
): MenuItem[] {
  return getActionsLike(filter).map((action) => menuItemFromAction(action, override));
}

//
// Context menus
//

export const CONTEXT_MENU_DEFAULT_FLOATING_OPTIONS: FloatingOptions = {
  placement: "bottom-right",
  referenceMargin: 0,
  containerMargin: 8,
};
export type ContextMenuInfo = MenuInfo & FloatingOptions & {};

/** The triggering element with some extra state */
type ContextMenuTriggerElement = HTMLElement & {
  contextMenuOnContextMenu?: (e: MouseEvent) => void;
};

type ContextMenuInstance = {
  id: number;
  info: ContextMenuInfo;
  trigger: HTMLElement;
  reference: { x: number; y: number };
  container?: HTMLElement | SVGElement;
};

const _activeContextMenu: Ref<ContextMenuInstance | null> = shallowRef(null);
export const activeContextMenu = pretendReadonly(_activeContextMenu);
let contextMenuId = 0;

export function createContextMenu(
  trigger: HTMLElement,
  reference: { x: number; y: number },
  info: ContextMenuInfo | ((ctx: MenuContext) => ContextMenuInfo),
): ContextMenuInstance {
  const container = findFloatingContainer(trigger) ?? undefined;
  const context = { triggerElement: trigger };
  const currentInfo: ContextMenuInfo = {
    ...CONTEXT_MENU_DEFAULT_FLOATING_OPTIONS,
    ...(typeof info === "function" ? info(context) : info),
    context,
  };
  const instance = { id: contextMenuId++, info: currentInfo, trigger, reference, container };
  _activeContextMenu.value = instance;
  trigger.dataset.contextmenu = "true";
  return instance;
}

export function destroyContextMenu(instance?: ContextMenuInstance) {
  if (instance == null || _activeContextMenu.value === instance) {
    if (activeContextMenu.value != null) {
      _activeContextMenu.value!.trigger.dataset.contextmenu = undefined;
    }
    _activeContextMenu.value = null;
  }
}

/** Simple context menu directive that creates a context menu on the element on click */
export const CONTEXTMENU_DIRECTIVE: Directive<
  MaybeElement,
  ContextMenuInfo | ((ctx: MenuContext) => ContextMenuInfo)
> = {
  mounted(el, binding) {
    const triggerEl = el as ContextMenuTriggerElement;
    triggerEl.contextMenuOnContextMenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      const reference = { x: e.clientX, y: e.clientY };
      createContextMenu(triggerEl, reference, binding.value);
    };
    triggerEl.addEventListener("contextmenu", triggerEl.contextMenuOnContextMenu);
  },

  unmounted(el) {
    const triggerEl = el as ContextMenuTriggerElement;
    triggerEl.removeEventListener("contextmenu", triggerEl.contextMenuOnContextMenu!);
  },
};
