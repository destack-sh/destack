import { HELPER_VIEW_TYPES, ROOT_VIEW_TYPES } from "@/language/const";
import { NodeReferenceData, NodeType, ObjectType, type AnyNodeData, type IconData, type ViewType } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { supergraph } from "@/system/globals";
import { canvas } from "@/system/space";
import {
  ACTION_BUILTIN_IDS_INDEX,
  fireAction,
  getAction,
  getActionsLike,
  getImplementingAction,
  getNodeActions,
  SCALAR_CONTEXT_ACTIONS,
  type Action,
  type ActionBuiltinId,
  type ActionContext,
  type ActionFilter,
} from "@/ui/action";
import { collectViewComponentsUp, findViewComponentUp, getVueComponentType, isViewComponentIn } from "@/ui/view";
import { getElement } from "@/utils/element";
import { type FloatingOptions } from "@/utils/floating";
import { log } from "@/utils/log";
import { pretendReadonly } from "@/utils/ref";
import { getViewTypeByComponentName, type ViewComponent } from "@/views/common";
import type { MaybeElement } from "@vueuse/core";
import { computed, shallowRef, toValue, triggerRef, type Directive, type Ref } from "vue";

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items?: MenuItem[];
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
  if (context?.element) {
    return collectViewComponentsUp(context.element);
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
  const implementation = getImplementingAction(action, contextViews, override?.context ?? {});
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
  element?: MaybeElement;
};

const POPOVER_DATA_SET_ATTRIBUTE = "popover";
const POPOVER_DATA_ID_ATTRIBUTE = "popoverid";

export const DEFAULT_POPOVER_FLOATING_OPTIONS: FloatingOptions = {
  placement: "bottom-right",
  referenceMargin: 4,
  containerMargin: 8,
};

export type PopoverContent =
  | ({ kind: "menu" } & MenuInfo & { width?: number; height?: number })
  | { kind: "view"; icon?: IconData; title?: string; component: any | ViewType; props: ViewComponent["props"] };
export type PopoverInfo = PopoverContent & {
  context?: PopoverContext;
  isEnabled?: boolean;
  reference?: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement;
  containerClass?: string;
  dontFocus?: boolean;
  dontAnimate?: boolean;
  onUpdate?(value?: any): void;
  onApply?(value?: any): void;
  onClose?(): void;
} & FloatingOptions;
export type PopoverInfoIn = Required<Pick<PopoverInfo, "kind">> & Partial<PopoverInfo> & PopoverContent;

/** The triggering element with some extra state */
type PopoverElement = HTMLElement & {
  menuOnEvent?: (e: MouseEvent) => void;
};

export type PopoverInstance = PopoverInfo & {
  id: number;
  generation: number;
  element?: HTMLElement | undefined; // the actual popover (set in PopoverOverlay)
  trigger: HTMLElement | SVGElement;
  reference: { x: number; y: number } | HTMLElement | SVGElement;
  container?: HTMLElement | SVGElement;
};

const _activePopovers: Ref<PopoverInstance[]> = shallowRef([]);
export const activePopovers = pretendReadonly(_activePopovers);
export const topPopover = computed(() => _activePopovers.value[_activePopovers.value.length - 1]);
export const hasActivePopover = computed(() => _activePopovers.value.length > 0);

let popoverGeneration = 0;
function newPopoverGeneration() {
  return popoverGeneration++;
}
let popoverId = 0;
function newPopoverId() {
  return popoverId++;
}

/** Pushes a popover onto the stack */
export function pushPopover(
  options: {
    generation?: number | "new";
    reference: { x: number; y: number } | HTMLElement | SVGElement;
    trigger: HTMLElement | SVGElement;
  } & PopoverInfoIn,
): PopoverInstance {
  // find container
  let container: HTMLElement | SVGElement | undefined;
  if (options.container == undefined) {
    const containingRoot = findViewComponentUp(options.trigger, (c) => isViewComponentIn(c, ROOT_VIEW_TYPES));
    container = containingRoot != null ? (getElement(containingRoot) ?? undefined) : undefined;
  } else {
    container = options.container;
  }

  // create popover instance
  let generation: number;
  if (options.generation == "new") {
    generation = newPopoverGeneration();
  } else if (options.generation == undefined) {
    generation = activePopovers.value[activePopovers.value.length - 1]?.generation ?? newPopoverGeneration();
  } else {
    generation = options.generation;
  }
  const instance: PopoverInstance = {
    id: newPopoverId(),
    ...DEFAULT_POPOVER_FLOATING_OPTIONS,
    ...options,
    generation,
    element: undefined,
    container,
  };
  _activePopovers.value.push(instance);
  triggerRef(_activePopovers);

  // mark trigger
  instance.trigger.dataset[POPOVER_DATA_SET_ATTRIBUTE] = "true";
  instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE] = instance.id.toString();

  log.trace("popover.push", instance);
  return instance;
}

/** Removes a popover from the stack */
export function popPopover(from: number | PopoverInstance = -1, generation?: number | undefined) {
  // figure out which popovers to close
  if (typeof from == "object") {
    generation = from.generation;
    from = _activePopovers.value.indexOf(from);
  }
  const closedPopovers = activePopovers.value
    .slice(from < 0 ? 0 : from)
    .filter((instance) => generation == undefined || instance.generation == generation);
  if (closedPopovers.length === 0) return;

  // close them
  closedPopovers.forEach((instance) => {
    if (instance != null && instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE] == instance?.id.toString()) {
      delete instance.trigger.dataset[POPOVER_DATA_SET_ATTRIBUTE];
      delete instance.trigger.dataset[POPOVER_DATA_ID_ATTRIBUTE];
    }
  });
  _activePopovers.value = _activePopovers.value.filter((instance) => !closedPopovers.some((p) => p.id == instance.id));
  log.trace("popover.pop", closedPopovers);
  return closedPopovers;
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
      const triggerEl = el as PopoverElement;
      triggerEl.menuOnEvent = (e: MouseEvent) => {
        const reference = options.reference == "self" ? triggerEl : { x: e.clientX, y: e.clientY };
        const node = canvas.findViewData(triggerEl) ?? undefined;
        const info =
          typeof binding.value == "function"
            ? binding.value({ element: triggerEl, nodes: node ? [node] : [] })
            : binding.value;
        if (info.isEnabled === false) return;
        e.preventDefault();
        e.stopPropagation();
        pushPopover({ trigger: triggerEl, ...info, reference: info.reference ?? reference });
      };
      triggerEl.addEventListener(options.event, triggerEl.menuOnEvent);
    },

    unmounted(el) {
      const triggerEl = el as PopoverElement;
      if (triggerEl.menuOnEvent) triggerEl.removeEventListener(options.event, triggerEl.menuOnEvent);
    },
  };
}

export const MENU_DIRECTIVE = makePopoverDirective({ event: "click", reference: "self" });

type HoverMenuElement = PopoverElement & {
  hoverTimeout?: number;
  hideTimeout?: number;
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

const DEFAULT_HOVER_DELAY = 800;
const DEFAULT_LEAVE_DELAY = 400;

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
    const triggerEl = el as HoverMenuElement;
    const options = binding.value;
    const hoverDelay = options.showDelay ?? DEFAULT_HOVER_DELAY;
    const leaveDelay = options.hideDelay ?? DEFAULT_LEAVE_DELAY;

    let isHovering = false;
    let popoverInstance: PopoverInstance | undefined;

    const showPopover = (e: MouseEvent) => {
      const reference = options.reference === "self" ? triggerEl : { x: e.clientX, y: e.clientY };
      const node = (window as any).canvas?.findViewData(triggerEl) ?? undefined;
      const popoverInfo =
        typeof options.popover === "function"
          ? options.popover({ element: triggerEl, nodes: [node] })
          : options.popover;
      if (popoverInfo.isEnabled === false) return;

      e.preventDefault();
      e.stopPropagation();
      popoverInstance = pushPopover({ ...popoverInfo, trigger: triggerEl, reference });
    };

    const hidePopover = () => {
      if (popoverInstance != null) {
        popPopover();
        popoverInstance = undefined;
      }
    };

    const startHoverTimer = (e: MouseEvent) => {
      clearTimeout(triggerEl.hoverTimeout);
      triggerEl.hoverTimeout = window.setTimeout(() => showPopover(e), hoverDelay);
    };

    const cancelShowTimer = () => {
      clearTimeout(triggerEl.hoverTimeout);
    };

    const startHideTimer = () => {
      clearTimeout(triggerEl.hideTimeout);
      triggerEl.hideTimeout = window.setTimeout(hidePopover, leaveDelay);
    };

    const cancelHideTimer = () => {
      clearTimeout(triggerEl.hideTimeout);
    };

    triggerEl.hoverOnMouseEnter = (e: MouseEvent) => {
      if (!isHovering) {
        const isEnabled = options.isEnabled != null ? options.isEnabled() : true;
        if (isEnabled) {
          isHovering = true;
          cancelHideTimer();
          startHoverTimer(e);
        }
      }
    };

    triggerEl.hoverOnMouseMove = (e: MouseEvent) => {
      if (isMouseOverlapping(e, triggerEl, popoverInstance?.element)) {
        if (!isHovering) {
          const isEnabled = options.isEnabled != null ? options.isEnabled() : true;
          if (isEnabled) {
            isHovering = true;
            cancelHideTimer();
            startHoverTimer(e);
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
    const triggerEl = el as HoverMenuElement;

    clearTimeout(triggerEl.hoverTimeout);
    clearTimeout(triggerEl.hideTimeout);

    if (triggerEl.hoverOnMouseEnter) {
      triggerEl.removeEventListener("mouseenter", triggerEl.hoverOnMouseEnter);
    }
    if (triggerEl.hoverOnMouseMove) {
      document.removeEventListener("mousemove", triggerEl.hoverOnMouseMove);
    }
  },
};

/**
 * Starts checking whether the mouse is in the given elements.
 * Stops tracking as soon as the 'hide' state is reached.
 *  Much like hover menu, but tied to an element on demand (instead of always).
 */
export function trackHoverElementOnce(
  triggerEl: HTMLElement,
  options: {
    hoverDelay?: number;
    hideDelay?: number;
    getOtherElements?: () => HTMLElement[];
    onHover?: () => void;
    onLeave?: () => void;
    immediate?: boolean; // start tracking immediately, stop on leave
  },
) {
  const {
    hoverDelay = DEFAULT_HOVER_DELAY,
    hideDelay = DEFAULT_LEAVE_DELAY,
    getOtherElements = () => [],
    onHover = () => {},
    onLeave = () => {},
  } = options;

  let isHovering = false;

  let hoverTimeout: number | undefined = undefined;
  let hideTimeout: number | undefined = undefined;

  const startHoverTimer = () => {
    clearTimeout(hoverTimeout);
    hoverTimeout = window.setTimeout(() => {
      onHover();
    }, hoverDelay);
  };
  const startHideTimer = () => {
    clearTimeout(hideTimeout);
    hideTimeout = window.setTimeout(() => {
      stopTracking();
      onLeave();
    }, hideDelay);
  };
  const cancelHoverTimer = () => {
    clearTimeout(hoverTimeout);
  };
  const cancelHideTimer = () => {
    clearTimeout(hideTimeout);
  };

  const onMouseMove = (e: MouseEvent) => {
    const elements = getOtherElements();
    if (isMouseOverlapping(e, triggerEl, ...elements)) {
      if (!isHovering) {
        isHovering = true;
        cancelHideTimer();
        startHoverTimer();
      }
    } else {
      if (isHovering) {
        isHovering = false;
        cancelHoverTimer();
        startHideTimer();
      }
    }
  };

  const startTracking = () => {
    document.addEventListener("mousemove", onMouseMove);
  };

  const stopTracking = () => {
    if (isHovering) {
      isHovering = false;
      cancelHoverTimer();
    }
    document.removeEventListener("mousemove", onMouseMove);
  };

  if (options.immediate) {
    startTracking();
  }

  return { startTracking, stopTracking };
}

/** Creates the default menu for the views at the given element. */
export function pushDefaultMenu(kind: "main" | "context", node: AnyNodeData | undefined, e: MouseEvent) {
  const hasSelection = canvas.selection != null && canvas.selection.nodesPtr.length > 1;
  const excludedActions = hasSelection ? SCALAR_CONTEXT_ACTIONS : [];

  let currentNode: AnyNodeData | null = node ?? null;
  const nodes: AnyNodeData[] = node != null ? [node] : [];
  const actionsById: Record<string, Action> = {};
  const nodeByActionId: Record<string, AnyNodeData> = {};
  const elementByActionId: Record<string, HTMLElement> = {};

  function addAction(element: HTMLElement, action: Action) {
    if (action.id != null && actionsById[action.id] == null) {
      if (excludedActions.includes(action.id)) return;
      actionsById[action.id] = action;
      if (currentNode != null) {
        nodeByActionId[action.id] = currentNode;
      }
      elementByActionId[action.id] = element;
    }
  }

  // add default action
  if (node != null) {
    const nodeActions = getNodeActions(node);
    nodeActions.forEach((action) => addAction(e.target as HTMLElement, action));
  }

  // gather stack of nodes and actions
  let element = e.target as HTMLElement;
  while (element != null) {
    // skip explicitly ignored elements
    if (element.dataset?.["contextmenu"] == "ignore") {
      element = element.parentElement!;
      continue;
    }
    // find new node at element
    let nextNodePtr: NodeReferenceData | null = null;
    let isNodeContainer = false; // whether the referenced node is a container
    if (element.dataset?.["nodeId"] != null) {
      nextNodePtr = {
        metatype: ObjectType.NODE_REFERENCE,
        nodeType: Number(element.dataset["nodeType"]),
        id: element.dataset["nodeId"],
        ck: element.dataset["nodeCk"],
      };
    } else if ((element as any).__viewComponent != null) {
      const component = (element as any).__viewComponent as ViewComponent;
      if (component.props.nodePtr != null) {
        nextNodePtr = component.props.nodePtr;
      }
      // check if the node is the container
      const componentType = getVueComponentType(component);
      const viewType = componentType != null ? getViewTypeByComponentName(componentType) : null;
      if (HELPER_VIEW_TYPES.has(viewType!)) {
        isNodeContainer = true;
      } else if (component.exposed.self.value != null) {
        const selfView = supergraph.get(component.exposed.self.value);
        if (HELPER_VIEW_TYPES.has(selfView?.type!)) {
          isNodeContainer = true;
        } else if (selfView?.parentPtr != null) {
          const parentView = supergraph.get(selfView.parentPtr);
          if (isNode(parentView, NodeType.VIEW) && ROOT_VIEW_TYPES.has(parentView.type)) {
            isNodeContainer = true;
          }
        }
      }
    }

    // add node
    let nextNode: AnyNodeData | null = null;
    if (nextNodePtr != null) {
      nextNode = supergraph.get(nextNodePtr);
      if (nextNode != null) {
        if (!nodes.some((node) => node.id == nextNode!.id)) {
          nodes.push(nextNode);
        }
        currentNode = nextNode;
      }
    }

    // gather actions
    const contextMenuItems = element.dataset["contextmenuItems"]?.split(",");
    if (contextMenuItems != null) {
      const contextMenuActions = getActionsLike(contextMenuItems);
      contextMenuActions.forEach((action) => addAction(element, action));
    }
    if (nextNode != null && !isNodeContainer) {
      const nodeActions = getNodeActions(nextNode);
      nodeActions.forEach((action) => addAction(element, action));
      // break; // NOTE :UX: maybe we should ignore other nodes to reduce possible confusion about which node the action relates to?
    }

    // and up we go
    element = element.parentElement!;
  }

  // build menu
  // NOTE :UX: associate actions with the most appropriate elements?
  //  (if we click on a Type view inside a Block selection, we want the list.create.above from the Block selection, not from the Type view)
  if (Object.keys(actionsById).length == 0 || nodes.length == 0) {
    return; // no actions found
  }
  if (!canvas.isSelected(nodes[0])) {
    // auto-select first node if not selected
    canvas.select([nodes[0]]);
  }
  const actions = Object.values(actionsById);
  actions.sort((a, b) => ACTION_BUILTIN_IDS_INDEX[a.id] - ACTION_BUILTIN_IDS_INDEX[b.id]);
  const context: PopoverContext = {
    event: e,
    element: e.target as HTMLElement,
    nodes: canvas.selection != null ? supergraph.getManyMaybe(canvas.selection.nodesPtr) : [nodes[0]],
  };
  const contextViews = getMenuContextViews(context);
  const menuItems = actions.map((action) => menuItemFromAction(action, { context, contextViews: contextViews }));
  pushPopover({
    kind: "menu",
    context,
    items: menuItems,
    trigger: e.target as HTMLElement,
    reference: { x: e.clientX, y: e.clientY },
  });
}

/** Creates the default menu for the given element. */
export function pushDefaultContextMenu(e: MouseEvent) {
  pushDefaultMenu("context", undefined, e);
}
