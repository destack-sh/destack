import type { IconData, TextData, Timestamp } from "@/proto/wire";
import type { Action } from "@/ui/action";
import { isOnMac } from "@/utils/browser";
import { findFloatingContainer, type FloatingOptions, type FloatingPlacement } from "@/utils/floating";
import { normalizeKeymapKey, parseKeymapSignature } from "@/ui/keymap";
import { log } from "@/utils/log";
import { pretendReadonly } from "@/utils/ref";
import { Casing, toCasing } from "@/utils/string";
import type { MaybeElement } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, shallowRef, toValue, type Directive, type FunctionalComponent, type Ref } from "vue";
import type { PopoverInfo, PopoverInfoIn } from "@/ui/popover";

//
// Shortcut
//

const IS_ON_MAC = isOnMac(window);
const KEY_ICONS_FA: Record<string, string | undefined> = {
  cmd: "fas fa-command",
  ctrl: IS_ON_MAC ? "fas fa-chevron-up" : undefined,
  mod: IS_ON_MAC ? "fas fa-command" : undefined, // :ModKey
  alt: IS_ON_MAC ? "fas fa-option" : undefined,
  enter: "fas fa-arrow-turn-down-left",
  backspace: "fas fa-delete-left",
  del: "fas fa-delete-left",
  tab: "fas fa-arrow-right-long-to-line",
  pageup: "fas fa-arrow-up-to-line",
  pagedown: "fas fa-arrow-down-to-line",
  up: "fas fa-arrow-up",
  down: "fas fa-arrow-down",
  left: "fas fa-arrow-left",
  right: "fas fa-arrow-right",
  home: "fas fa-house",
  end: "fas fa-flag",
};
const KEY_ICONS_TEXT: Record<string, string> = {
  shift: "⇧",
};
export const Shortcut: FunctionalComponent<{ shortcut: string }> = (props) => {
  const parsed = computed(() => parseKeymapSignature(props.shortcut));
  return (
    // Shortcut
    <span class="flex select-none flex-row gap-x-2">
      {parsed.value.chords.map((chord) => {
        const keys = [...chord.modifiers, chord.key];
        return (
          // Chord
          <span class="flex flex-row gap-x-0.5">
            {keys.map((key) => (
              // Key
              <kbd class="min-w-5 rounded border border-gray-300 bg-white px-1 py-0.5 text-center font-sans text-xs  hover:border-orange-900 hover:bg-gray-100 hover:text-orange-900">
                {KEY_ICONS_FA[key] != null ? (
                  <i class={KEY_ICONS_FA[key]} />
                ) : KEY_ICONS_TEXT[key] != null ? (
                  KEY_ICONS_TEXT[key]
                ) : (
                  toCasing(normalizeKeymapKey(key), Casing.CAMEL)
                )}
              </kbd>
            ))}
          </span>
        );
      })}
    </span>
  );
};

//
// Tooltip
//

const DEFAULT_HOVER_SHOW_DELAY = 500;
const DEFAULT_HOVER_HIDE_DELAY = 300;

export type TooltipInfo = Omit<FloatingOptions, "placement"> & {
  icon?: string | IconData;
  title?: string | (() => string);
  text: string | TextData | (() => string | TextData);
  small?: boolean;
  shortcuts?: string[];
  showDelay?: number;
  hideDelay?: number;
  placement?: FloatingPlacement;
  isEnabled?: boolean | (() => boolean);
  group?: string;
};

export function tooltipFromAction(action: Action, override?: Partial<TooltipInfo>): TooltipInfo {
  return {
    icon: action.icon?.faName,
    title: toValue(action.title),
    text: action.text,
    shortcuts: action.shortcuts,
    placement: "top",
    ...override,
  };
}

/** The triggering element with some extra state */
interface TooltipTriggerElement extends HTMLElement {
  tooltipInstance?: TooltipInstance;
  tooltipShowTimeout?: number;
  tooltipHideTimeout?: number;
  tooltipOnMouseEnter?: (e: MouseEvent) => void;
  tooltipOnMouseLeave?: (e: MouseEvent) => void;
}

/** An active instance of a tooltip */
export type TooltipInstance = {
  id: number;
  info: TooltipInfo;
  reference: TooltipTriggerElement;
  container?: HTMLElement | SVGElement;
  createdAt: DateTime;
};

const _activeTooltips: Ref<TooltipInstance[]> = shallowRef([]);
export const activeTooltips = pretendReadonly(_activeTooltips);
let lastActiveTooltip: TooltipInstance | undefined;

const TOOLTIP_DATA_SET_ATTRIBUTE = "tooltip";
const TOOLTIP_DATA_ID_ATTRIBUTE = "tooltipid";

let tooltipId = 0;
function createTooltip(
  reference: TooltipTriggerElement,
  info: TooltipInfo,
  container: HTMLElement | SVGElement | undefined,
): TooltipInstance {
  const instance = { id: tooltipId++, info, reference, container, createdAt: DateTime.now() };
  _activeTooltips.value = [..._activeTooltips.value, instance];
  reference.dataset[TOOLTIP_DATA_SET_ATTRIBUTE] = "true";
  reference.dataset[TOOLTIP_DATA_ID_ATTRIBUTE] = instance.id.toString();
  reference.tooltipInstance = instance;
  lastActiveTooltip = instance;
  log.trace("tooltip.create", instance);
  return instance;
}

function destroyTooltip(instance: TooltipInstance) {
  _activeTooltips.value = _activeTooltips.value.filter((t) => t !== instance);
  lastActiveTooltip = _activeTooltips.value[_activeTooltips.value.length - 1];
  if (instance.reference.dataset[TOOLTIP_DATA_ID_ATTRIBUTE] == instance.id.toString()) {
    delete instance.reference.dataset[TOOLTIP_DATA_SET_ATTRIBUTE];
    delete instance.reference.dataset[TOOLTIP_DATA_ID_ATTRIBUTE];
  }
  instance.reference.tooltipInstance = undefined;
}

/** Simple tooltip directive that shows/hides itself on hover with a delay*/
export const TOOLTIP_DIRECTIVE: Directive<MaybeElement, TooltipInfo> = {
  mounted(el, binding) {
    const triggerEl = el as TooltipTriggerElement;

    triggerEl.tooltipOnMouseEnter = (e: MouseEvent) => {
      const info = binding.value;
      if (e.target == triggerEl) {
        // create a new tooltip instance if trigger is hovered for a while or its group was recently hovered
        const container = findFloatingContainer(triggerEl) ?? undefined;
        if (triggerEl.tooltipShowTimeout != null) {
          clearTimeout(triggerEl.tooltipShowTimeout);
        }
        if (
          info.group != null &&
          info.group == lastActiveTooltip?.info?.group &&
          lastActiveTooltip.createdAt.diffNow().milliseconds < 500
        ) {
          destroyTooltip(lastActiveTooltip!);
          createTooltip(triggerEl, info, container);
        } else {
          triggerEl.tooltipShowTimeout = window.setTimeout(() => {
            if (info.isEnabled == null || toValue(info.isEnabled)) {
              createTooltip(triggerEl, info, container);
            }
          }, info.showDelay ?? DEFAULT_HOVER_SHOW_DELAY);
        }
      } else if (triggerEl.tooltipInstance != null) {
        // some part of the tooltip is hovered, so cancel the hide timeout
        if (triggerEl.tooltipHideTimeout != null) {
          clearTimeout(triggerEl.tooltipHideTimeout);
        }
      }
    };
    triggerEl.tooltipOnMouseLeave = (e: MouseEvent) => {
      if (triggerEl.tooltipShowTimeout != null) {
        clearTimeout(triggerEl.tooltipShowTimeout);
      }
      const info = binding.value;
      triggerEl.tooltipHideTimeout = window.setTimeout(() => {
        if (triggerEl.tooltipInstance != null) {
          destroyTooltip(triggerEl.tooltipInstance);
        }
      }, info.hideDelay ?? DEFAULT_HOVER_HIDE_DELAY);
    };
    triggerEl.addEventListener("mouseenter", triggerEl.tooltipOnMouseEnter);
    triggerEl.addEventListener("mouseleave", triggerEl.tooltipOnMouseLeave);
  },

  updated(el, binding) {
    const tooltipEl = el as TooltipTriggerElement;
    if (tooltipEl.tooltipInstance != null) {
      tooltipEl.tooltipInstance.info = binding.value;
    }
  },

  unmounted(el) {
    const triggerEl = el as TooltipTriggerElement;
    if (triggerEl.tooltipShowTimeout != null) clearTimeout(triggerEl.tooltipShowTimeout);
    if (triggerEl.tooltipHideTimeout != null) clearTimeout(triggerEl.tooltipHideTimeout);

    if (triggerEl.tooltipInstance != null)
      _activeTooltips.value = _activeTooltips.value.filter((t) => t !== triggerEl.tooltipInstance);

    if (triggerEl.tooltipOnMouseEnter) triggerEl.removeEventListener("mouseenter", triggerEl.tooltipOnMouseEnter);
    if (triggerEl.tooltipOnMouseLeave) triggerEl.removeEventListener("mouseleave", triggerEl.tooltipOnMouseLeave);
  },
};

//
// Hover directive
//

export type HoverInfo = {
  showDelay?: number;
  hideDelay?: number;
  popover?: () => PopoverInfoIn;
  show?: (e: MouseEvent) => void;
  hide?: (e: MouseEvent) => void;
};

type HoverTriggerElement = {
  hoverShowTimeout?: number;
  hoverHideTimeout?: number;
  hoverOnMouseEnter?: (e: MouseEvent) => void;
  hoverOnMouseLeave?: (e: MouseEvent) => void;
} & HTMLElement;

/** Convenience hover directive to perform arbitrary actions */
export const HOVER_DIRECTIVE: Directive<MaybeElement, HoverInfo> = {
  mounted(el, binding) {
    const triggerEl = el as HoverTriggerElement;
    const { showDelay = DEFAULT_HOVER_SHOW_DELAY, hideDelay = DEFAULT_HOVER_HIDE_DELAY } = binding.value;

    triggerEl.hoverOnMouseEnter = (e: MouseEvent) => {
      if (e.target == triggerEl) {
        if (triggerEl.hoverShowTimeout != null) clearTimeout(triggerEl.hoverShowTimeout);
        triggerEl.hoverShowTimeout = window.setTimeout(() => {
          binding.value.show?.(e);
        }, showDelay);
      } else {
        if (triggerEl.hoverHideTimeout != null) clearTimeout(triggerEl.hoverHideTimeout);
      }
    };
    triggerEl.hoverOnMouseLeave = (e: MouseEvent) => {
      if (triggerEl.hoverShowTimeout != null) clearTimeout(triggerEl.hoverShowTimeout);
      triggerEl.hoverHideTimeout = window.setTimeout(() => {
        binding.value.hide?.(e);
      }, hideDelay);
    };
    triggerEl.addEventListener("mouseenter", triggerEl.hoverOnMouseEnter);
    triggerEl.addEventListener("mouseleave", triggerEl.hoverOnMouseLeave);
  },

  updated(el, binding) {
    const triggerEl = el as HoverTriggerElement;
    if (triggerEl.hoverShowTimeout != null) clearTimeout(triggerEl.hoverShowTimeout);
    if (triggerEl.hoverHideTimeout != null) clearTimeout(triggerEl.hoverHideTimeout);
  },

  unmounted(el) {
    const triggerEl = el as HoverTriggerElement;
    if (triggerEl.hoverShowTimeout != null) clearTimeout(triggerEl.hoverShowTimeout);
    if (triggerEl.hoverHideTimeout != null) clearTimeout(triggerEl.hoverHideTimeout);
    if (triggerEl.hoverOnMouseEnter) triggerEl.removeEventListener("mouseenter", triggerEl.hoverOnMouseEnter);
    if (triggerEl.hoverOnMouseLeave) triggerEl.removeEventListener("mouseleave", triggerEl.hoverOnMouseLeave);
  },
};

//
// Input event outside directive
//

const INPUT_EVENTS = [
  "click",
  "mousedown",
  "mouseup",
  "mouseenter",
  "mouseleave",
  "touchstart",
  "touchend",
  "keydown",
  "keyup",
  "input",
  "change",
  "focus",
  "blur",
  "contextmenu",
];
type InputEventName = (typeof INPUT_EVENTS)[number];
type InputOutsideCallback = (e: Event) => boolean;

type EventOutsideTriggerElement = {
  inputOutsideEventName?: InputEventName;
  inputOutsideOnInput?: (e: Event) => void;
} & HTMLElement;

/** Convenience X outside Y directive. Event name is derived from modifier. */
export const EVENT_OUTSIDE_DIRECTIVE: Directive<MaybeElement, InputOutsideCallback> = {
  mounted(el, binding) {
    const triggerEl = el as EventOutsideTriggerElement;
    const eventName = Object.keys(binding.modifiers)[0];
    if (!INPUT_EVENTS.includes(eventName)) throw new Error(`Invalid input event: ${eventName}`);

    triggerEl.inputOutsideOnInput = (e: Event) => {
      if (!triggerEl.contains(e.target as Node)) {
        binding.value!(e);
        if (binding.modifiers.stop) e.stopPropagation();
        if (binding.modifiers.prevent) e.preventDefault();
      }
    };
    triggerEl.inputOutsideEventName = eventName;
    document.addEventListener(eventName, triggerEl.inputOutsideOnInput);
  },

  unmounted(el) {
    const triggerEl = el as EventOutsideTriggerElement;
    if (triggerEl.inputOutsideOnInput)
      document.removeEventListener(triggerEl.inputOutsideEventName!, triggerEl.inputOutsideOnInput);
  },
};
