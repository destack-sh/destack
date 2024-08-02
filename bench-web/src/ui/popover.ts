import type { IconData, ViewType } from "@/proto/wire";
import {
  fireAction,
  getAction,
  getActionsLike,
  getImplementingAction,
  type Action,
  type ActionBuiltinId,
  type ActionContext,
  type ActionFilter,
} from "@/ui/action";
import { ROOT_VIEW_TYPES } from "@/language/utils";
import { canvas } from "@/system/space";
import { getElement } from "@/utils/element";
import { type FloatingOptions } from "@/utils/floating";
import { pretendReadonly } from "@/utils/ref";
import { collectViewComponentsUp, findViewComponentUp, isViewComponentIn } from "@/ui/canvas";
import type { ViewComponent } from "@/views/common";
import type { MaybeElement } from "@vueuse/core";
import { computed, shallowRef, toValue, triggerRef, type Directive, type Ref } from "vue";

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items: MenuItem[];
  context?: PopoverContext;
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
function getMenuContextViews(context?: PopoverContext): ViewComponent[] {
  if (context?.triggerElement) {
    return collectViewComponentsUp(context.triggerElement);
  } else {
    return canvas.focusedViewComponents;
  }
}

/** Maps an action to a typical menu item in context  */
export function menuItemFromAction(
  actionOrId: Action | ActionBuiltinId,
  override?: Partial<MenuItem> & { context?: PopoverContext; contextViews?: ViewComponent[] },
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
  override: Partial<MenuItem> & { context: PopoverContext | undefined },
): MenuItem[] {
  const contextViews = getMenuContextViews(override?.context);
  const actions = getActionsLike(filter).map((action) =>
    menuItemFromAction(action, { contextViews, ...(override ?? {}) }),
  );
  return actions;
}

//
// Popover stack
//

export type PopoverContext = ActionContext & {
  triggerElement?: MaybeElement;
};

const POPOVER_DATA_SET_ATTRIBUTE = "popover";
const POPOVER_DATA_ID_ATTRIBUTE = "popoverid";

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
      propsRef: () => ViewComponent["props"];
      context?: PopoverContext;
    }
) & {
  isEnabled?: boolean;
  reference?: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement | "containingRoot";
  containerClass?: string;
  dontFocus?: boolean;
  onUpdate?(value?: any): void;
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
  element: HTMLElement | undefined; // the actual popover (set in PopoverOverlay)
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

export function updatePopover(instance: PopoverInstance, info: Partial<PopoverInfo>) {
  const idx = _activePopovers.value.indexOf(instance);
  if (idx < 0) return;
  instance.info = { ...instance.info, ...info } as PopoverInfo;
  _activePopovers.value[idx] = instance;
  triggerRef(_activePopovers);
}

export function pushPopover(push: {
  trigger: HTMLElement | SVGElement;
  reference: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement | undefined;
  info: PopoverInfoIn;
}): PopoverInstance {
  const triggerNode = canvas.findViewData(push.trigger) ?? undefined;
  const context: PopoverContext = { triggerElement: push.trigger, triggerNode };
  const info = {
    ...OVERLAY_MENU_DEFAULT_FLOATING_OPTIONS,
    ...push.info,
    context,
    kind: "component" in push.info ? "component" : "menu",
  } as PopoverInfo;

  let container: HTMLElement | SVGElement | undefined;
  if (info.container == "containingRoot") {
    const containingRoot = findViewComponentUp(push.trigger, (c) => isViewComponentIn(c, ROOT_VIEW_TYPES));
    if (containingRoot == null) throw new Error("no containing root found");
    container = getElement(containingRoot) ?? undefined;
  } else {
    container = info.container ?? container;
  }

  const instance: PopoverInstance = {
    id: newPopoverId(),
    element: undefined, // set in PopoverOverlay
    info,
    trigger: push.trigger,
    reference: info.reference ?? push.reference,
    container,
  };
  _activePopovers.value.push(instance);
  triggerRef(_activePopovers);
  instance.trigger.dataset[POPOVER_DATA_SET_ATTRIBUTE] = "true";
  instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE] = instance.id.toString();
  return instance;
}

export function popPopover(fromIdx: number = -1) {
  const closedMenus = activePopovers.value.slice(fromIdx < 0 ? 0 : fromIdx);
  if (closedMenus.length === 0) return;
  closedMenus.forEach((instance) => {
    if (instance != null && instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE] == instance?.id.toString()) {
      delete instance.trigger.dataset[POPOVER_DATA_SET_ATTRIBUTE];
      delete instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE];
    }
  });
  if (fromIdx >= 0) _activePopovers.value = _activePopovers.value.slice(0, fromIdx);
  else _activePopovers.value = [];
  return closedMenus;
}

//
// Popover directives
//

function makePopoverDirective(options: {
  event: "contextmenu" | "click";
  reference: "trigger" | "self";
}): Directive<MaybeElement, PopoverInfoIn | ((context: PopoverContext) => PopoverInfoIn)> {
  return {
    mounted(el, binding) {
      const triggerEl = el as PopoverTriggerElement;
      triggerEl.menuOnEvent = (e: MouseEvent) => {
        const reference = options.reference == "self" ? triggerEl : { x: e.clientX, y: e.clientY };
        const triggerNode = canvas.findViewData(triggerEl) ?? undefined;
        const info =
          typeof binding.value == "function"
            ? binding.value({ triggerElement: triggerEl, triggerNode })
            : binding.value;
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

type HoverMenuTriggerElement = PopoverTriggerElement & {
  hoverShowTimeout?: number;
  hoverHideTimeout?: number;
  hoverOnMouseEnter?: (e: MouseEvent) => void;
  hoverOnMouseMove?: (e: MouseEvent) => void;
};

export type HoverMenuOptions = {
  showDelay?: number;
  hideDelay?: number;
  reference: "trigger" | "self";
  isEnabled?: () => boolean;
  popover: (context: PopoverContext) => PopoverInfoIn;
};

const DEFAULT_SHOW_DELAY = 300;
const DEFAULT_HIDE_DELAY = 300;

/** Checks whether the cursor position is overlapping any of the given elements. */
function isMouseOverlapping(e: MouseEvent, ...els: (HTMLElement | undefined)[]): boolean {
  for (const el of els) {
    if (el == null) continue;
    const rect = el.getBoundingClientRect();
    if (e.clientX >= rect.left && e.clientX <= rect.right && e.clientY >= rect.top && e.clientY <= rect.bottom) {
      return true;
    }
  }
  return false;
}

export const HOVER_MENU_DIRECTIVE: Directive<MaybeElement, HoverMenuOptions> = {
  mounted(el, binding) {
    const triggerEl = el as HoverMenuTriggerElement;
    const options = binding.value;
    const showDelay = options.showDelay ?? DEFAULT_SHOW_DELAY;
    const hideDelay = options.hideDelay ?? DEFAULT_HIDE_DELAY;

    let isHovering = false;
    let popoverInstance: PopoverInstance | undefined;

    const showPopover = (e: MouseEvent) => {
      const reference = options.reference === "self" ? triggerEl : { x: e.clientX, y: e.clientY };
      const triggerNode = (window as any).canvas?.findViewData(triggerEl) ?? undefined;
      const popoverInfo =
        typeof options.popover === "function"
          ? options.popover({ triggerElement: triggerEl, triggerNode })
          : options.popover;
      if (popoverInfo.isEnabled === false) return;

      e.preventDefault();
      e.stopPropagation();
      popoverInstance = pushPopover({ trigger: triggerEl, reference, info: popoverInfo });
    };

    const hidePopover = () => {
      if (popoverInstance != null) {
        popPopover();
        popoverInstance = undefined;
      }
    };

    const startShowTimer = (e: MouseEvent) => {
      clearTimeout(triggerEl.hoverShowTimeout);
      triggerEl.hoverShowTimeout = window.setTimeout(() => showPopover(e), showDelay);
    };

    const cancelShowTimer = () => {
      clearTimeout(triggerEl.hoverShowTimeout);
    };

    const startHideTimer = () => {
      clearTimeout(triggerEl.hoverHideTimeout);
      triggerEl.hoverHideTimeout = window.setTimeout(hidePopover, hideDelay);
    };

    const cancelHideTimer = () => {
      clearTimeout(triggerEl.hoverHideTimeout);
    };

    triggerEl.hoverOnMouseEnter = (e: MouseEvent) => {
      isHovering = true;
      cancelHideTimer();
      startShowTimer(e);
    };

    triggerEl.hoverOnMouseMove = (e: MouseEvent) => {
      if (isMouseOverlapping(e, triggerEl, popoverInstance?.element)) {
        if (!isHovering) {
          const isEnabled = options.isEnabled?.() !== false;
          if (isEnabled) {
            isHovering = true;
            cancelHideTimer();
            startShowTimer(e);
          }
        }
      } else {
        if (isHovering) {
          isHovering = false;
          cancelShowTimer();
          startHideTimer();
        }
      }
    };

    triggerEl.addEventListener("mouseenter", triggerEl.hoverOnMouseEnter);
    document.addEventListener("mousemove", triggerEl.hoverOnMouseMove);
  },

  unmounted(el) {
    const triggerEl = el as HoverMenuTriggerElement;

    clearTimeout(triggerEl.hoverShowTimeout);
    clearTimeout(triggerEl.hoverHideTimeout);

    if (triggerEl.hoverOnMouseEnter) {
      triggerEl.removeEventListener("mouseenter", triggerEl.hoverOnMouseEnter);
    }
    if (triggerEl.hoverOnMouseMove) {
      document.removeEventListener("mousemove", triggerEl.hoverOnMouseMove);
    }
  },
};
