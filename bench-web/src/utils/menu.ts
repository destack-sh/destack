import type { IconData, TextData, ViewType } from "@/proto/wire";
import {
  fireAction,
  getAction,
  getActionsLike,
  getImplementingAction,
  type Action,
  type ActionBuiltinId,
  type ActionContext,
  type ActionFilter,
} from "@/system/action";
import { canvas } from "@/system/space";
import { type FloatingOptions } from "@/utils/floating";
import { pretendReadonly } from "@/utils/ref";
import { collectViewComponentsUp } from "@/views/canvas";
import type { ViewComponent } from "@/views/common";
import type { MaybeElement } from "@vueuse/core";
import { computed, shallowRef, toValue, triggerRef, type Directive, type Ref } from "vue";

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
  const implementation = getImplementingAction(action, contextViews);
  let isChecked = undefined;
  if (action.type == "toggle") {
    if (typeof implementation?.isChecked == "object") {
      isChecked = implementation.isChecked.value;
    } else if (typeof implementation?.isChecked == "function") {
      isChecked = implementation.isChecked(action, override?.context);
    }
  }

  // map to action
  return {
    id: action.id,
    type: MENU_ITEM_TYPES.includes(action.type as any) ? (action.type as MenuItemType) : "generic",
    icon: action.icon?.faName,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled: implementation == null,
    isChecked,
    // default to subcategory since we usually group menus by category(ish)
    category: `${action.category}.${action.subcategory}`,
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
  filter: ActionFilter | string[],
  override?: Partial<MenuItem> & { context?: MenuContext },
): MenuItem[] {
  const contextViews = getMenuContextViews(override?.context);
  const actions = getActionsLike(filter).map((action) =>
    menuItemFromAction(action, { contextViews, ...(override ?? {}) }),
  );
  return actions;
}

//
// Overlay menus
//

const MENU_DATA_SET_ATTRIBUTE = "menu";
const MENU_DATA_ID_ATTRIBUTE = "menuid";

export const OVERLAY_MENU_DEFAULT_FLOATING_OPTIONS: FloatingOptions = {
  placement: "bottom-right",
  referenceMargin: 4,
  containerMargin: 8,
};

export type PopoverInfo = (
  | ({ kind: "menu" } & MenuInfo)
  | {
      kind: "component";
      component: any | ViewType;
      props: ViewComponent["props"];
      context?: MenuContext;
    }
) & {
  isEnabled?: boolean;
  reference?: { x: number; y: number } | HTMLElement | SVGElement;
  containerClass?: string;
  dontFocus?: boolean;
  onApply?(value?: any): void;
  onClose?(): void;
} & FloatingOptions;
export type PopoverInfoIn = Partial<PopoverInfo>;

/** The triggering element with some extra state */
type PopoverTriggerElement = HTMLElement & {
  menuOnEvent?: (e: MouseEvent) => void;
};

export type PopoverInstance = {
  id: number;
  info: PopoverInfo;
  trigger: HTMLElement | SVGElement;
  reference: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement;
};

const _activePopovers: Ref<PopoverInstance[]> = shallowRef([]);
export const activePopovers = pretendReadonly(_activePopovers);
export const topPopover = computed(() => _activePopovers.value[_activePopovers.value.length - 1]);
export const hasActivePopover = computed(() => _activePopovers.value.length > 0);

let PopoverId = 0;
function newPopoverId() {
  return PopoverId++;
}

export function pushPopover(create: {
  trigger: HTMLElement | SVGElement;
  reference: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement | undefined;
  info: PopoverInfoIn;
}): PopoverInstance {
  const { trigger, container } = create;
  const triggerNode = canvas.findViewData(trigger) ?? undefined;
  const context: MenuContext = { triggerElement: trigger, triggerNode };
  const info = {
    ...OVERLAY_MENU_DEFAULT_FLOATING_OPTIONS,
    ...create.info,
    context,
    kind: "component" in create.info ? "component" : "menu",
  } as PopoverInfo;

  // the info's reference is useful when overriding the actual reference in a directive
  const instance = { id: newPopoverId(), info, trigger, reference: info.reference ?? create.reference, container };
  _activePopovers.value.push(instance);
  triggerRef(_activePopovers);
  trigger.dataset[MENU_DATA_SET_ATTRIBUTE] = "true";
  trigger.dataset[MENU_DATA_ID_ATTRIBUTE] = instance.id.toString();
  return instance;
}

export function popPopover(fromIdx: number = -1) {
  const closedMenus = activePopovers.value.slice(fromIdx < 0 ? 0 : fromIdx);
  if (closedMenus.length === 0) return;
  closedMenus.forEach((instance) => {
    if (instance != null && instance.trigger.dataset[MENU_DATA_ID_ATTRIBUTE] == instance?.id.toString()) {
      delete instance.trigger.dataset[MENU_DATA_SET_ATTRIBUTE];
      delete instance.trigger.dataset[MENU_DATA_ID_ATTRIBUTE];
    }
  });
  if (fromIdx >= 0) _activePopovers.value = _activePopovers.value.slice(0, fromIdx);
  else _activePopovers.value = [];
  return closedMenus;
}

function makePopoverDirective(options: {
  event: "contextmenu" | "click";
  reference: "trigger" | "self";
}): Directive<MaybeElement, PopoverInfoIn | (() => PopoverInfoIn)> {
  return {
    mounted(el, binding) {
      const triggerEl = el as PopoverTriggerElement;
      triggerEl.menuOnEvent = (e: MouseEvent) => {
        const reference = options.reference == "self" ? triggerEl : { x: e.clientX, y: e.clientY };
        const info = typeof binding.value == "function" ? binding.value() : binding.value;
        if (info.isEnabled === false) return;
        e.preventDefault();
        e.stopPropagation();
        pushPopover({ trigger: triggerEl, reference, info });
      };
      triggerEl.addEventListener(options.event, triggerEl.menuOnEvent);
    },

    unmounted(el) {
      const triggerEl = el as PopoverTriggerElement;
      if (triggerEl.menuOnEvent) triggerEl.removeEventListener(options.event, triggerEl.menuOnEvent);
    },
  };
}

export const CONTEXT_MENU_DIRECTIVE = makePopoverDirective({ event: "contextmenu", reference: "trigger" });
export const MENU_DIRECTIVE = makePopoverDirective({ event: "click", reference: "self" });
