import type { IconData } from "@/proto/wire";
import {
  fireAction,
  getAction,
  type Action,
  type ActionBuiltinId,
  type ActionFilter,
  getActionsLike,
} from "@/system/action";
import { findFloatingContainer, type FloatingOptions } from "@/utils/floating";
import { pretendReadonly } from "@/utils/ref";
import type { MaybeElement } from "@vueuse/core";
import { shallowRef, toValue, type Ref, type Directive } from "vue";

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items: MenuItem[];
};

export type MenuItem = {
  id: string;
  icon?: string | IconData;
  title: string;
  category?: string;
  shortcuts?: string[];
  isDisabled?: boolean;
  isLoading?: boolean;
  action: (() => void) | MenuInfo;
};

export function menuItemFromAction(actionOrId: Action | ActionBuiltinId, override?: Partial<MenuItem>): MenuItem {
  const action = typeof actionOrId === "string" ? getAction(actionOrId) : actionOrId;
  return {
    id: action.id,
    icon: action.icon?.name,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled: action.enabled != null && !action.enabled.value,
    category: action.category,
    action: () => fireAction(action),
    ...override,
  };
}

/** Convenience wrapper around action filter & menu item mapping */
export function menuActionsLike(filter: ActionFilter, override?: Partial<MenuItem>): MenuItem[] {
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
  reference: { x: number; y: number };
  container?: HTMLElement | SVGElement;
};

const _activeContextMenu: Ref<ContextMenuInstance | null> = shallowRef(null);
export const activeContextMenu = pretendReadonly(_activeContextMenu);
let contextMenuId = 0;

export function createContextMenu(
  trigger: HTMLElement,
  reference: { x: number; y: number },
  info: ContextMenuInfo,
): ContextMenuInstance {
  const container = findFloatingContainer(trigger) ?? undefined;
  const instance = { id: contextMenuId++, info, reference, container };
  _activeContextMenu.value = instance;
  return instance;
}

export function destroyContextMenu(instance?: ContextMenuInstance) {
  if (instance == null || _activeContextMenu.value === instance) {
    _activeContextMenu.value = null;
  }
}

/** Simple context menu directive that creates a context menu on the element on click */
export const CONTEXT_MENU_DIRECTIVE: Directive<MaybeElement, ContextMenuInfo | (() => ContextMenuInfo)> = {
  mounted(el, binding) {
    const triggerEl = el as ContextMenuTriggerElement;
    const info = { ...CONTEXT_MENU_DEFAULT_FLOATING_OPTIONS, ...toValue(binding.value) };
    triggerEl.contextMenuOnContextMenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      const reference = { x: e.clientX, y: e.clientY };
      createContextMenu(triggerEl, reference, info);
    };
    triggerEl.addEventListener("contextmenu", triggerEl.contextMenuOnContextMenu);
  },

  unmounted(el) {
    const triggerEl = el as ContextMenuTriggerElement;
    triggerEl.removeEventListener("contextmenu", triggerEl.contextMenuOnContextMenu!);
  },
};
