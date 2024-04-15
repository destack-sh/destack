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
import { collectViewComponentsUp, getVueComponentType } from "@/views/canvas";
import type { MaybeElement } from "@vueuse/core";
import { shallowRef, toValue, type Directive, type Ref } from "vue";

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
    icon: action.icon?.faName,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled,
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

const CONTEXT_MENU_EVENTS = [
  "click",
  "mousedown",
  "contextmenu",
]

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
  container: HTMLElement | SVGElement | undefined,
  info: ContextMenuInfo | ((ctx: MenuContext) => ContextMenuInfo),
): ContextMenuInstance {
  const triggerNode = canvas.findViewData(trigger) ?? undefined;
  const context: MenuContext = { triggerElement: trigger, triggerNode };
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
export const CONTEXTMENU_DIRECTIVE: Directive<MaybeElement, ContextMenuInfo | ((ctx: MenuContext) => ContextMenuInfo)> =
  {
    mounted(el, binding) {
      const triggerEl = el as ContextMenuTriggerElement;
      triggerEl.contextMenuOnContextMenu = (e) => {
        e.preventDefault();
        e.stopPropagation();
        const reference = { x: e.clientX, y: e.clientY };
        createContextMenu(triggerEl, reference, undefined, binding.value);
      };
      triggerEl.addEventListener("contextmenu", triggerEl.contextMenuOnContextMenu);
    },

    unmounted(el) {
      const triggerEl = el as ContextMenuTriggerElement;
      triggerEl.removeEventListener("contextmenu", triggerEl.contextMenuOnContextMenu!);
    },
  };
