import type { TextData } from "@/proto/wire";
import type { Action } from "@/system/action";
import { isOnMac } from "@/utils/browser";
import { findFloatingContainer, type FloatingOptions, type FloatingPlacement } from "@/utils/floating";
import { normalizeKeymapKey, parseKeymapSignature } from "@/utils/keymap";
import { pretendReadonly } from "@/utils/ref";
import { Casing, toCasing } from "@/utils/string";
import type { MaybeElement } from "@vueuse/core";
import { computed, shallowRef, toValue, type Directive, type FunctionalComponent, type Ref } from "vue";

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
              <kbd class="min-w-5 rounded-md border border-gray-300 bg-white px-1 py-0.5 text-center font-sans text-xs  hover:border-orange-900 hover:bg-gray-100 hover:text-orange-900">
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
  icon?: string;
  title?: string;
  text: string | TextData;
  arrow?: boolean;
  shortcuts?: string[];
  showDelay?: number;
  hideDelay?: number;
  placement?: FloatingPlacement;
};

export function tooltipFromAction(action: Action): TooltipInfo {
  return {
    icon: action.icon?.name,
    title: toValue(action.title),
    text: action.text,
    shortcuts: action.shortcuts,
    arrow: true,
    placement: "top",
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
};

const _activeTooltips: Ref<TooltipInstance[]> = shallowRef([]);
export const activeTooltips = pretendReadonly(_activeTooltips);
let tooltipId = 0;

function createTooltip(
  reference: TooltipTriggerElement,
  info: TooltipInfo,
  container: HTMLElement | SVGElement | undefined,
): TooltipInstance {
  const instance = { id: tooltipId++, info, reference, container };
  _activeTooltips.value = [..._activeTooltips.value, instance];
  return instance;
}

function destroyTooltip(instance: TooltipInstance) {
  _activeTooltips.value = _activeTooltips.value.filter((t) => t !== instance);
}

/** Simple tooltip directive that shows/hides itself on hover with a delay*/
export const TOOLTIP_DIRECTIVE: Directive<MaybeElement, TooltipInfo> = {
  mounted(el, binding) {
    const triggerEl = el as TooltipTriggerElement;
    const { showDelay = DEFAULT_HOVER_SHOW_DELAY, hideDelay = DEFAULT_HOVER_HIDE_DELAY } = binding.value;

    triggerEl.tooltipOnMouseEnter = (e: MouseEvent) => {
      if (e.target == triggerEl) {
        // create a new tooltip instance if trigger is hovered for a while
        const container = findFloatingContainer(triggerEl) ?? undefined;
        if (triggerEl.tooltipShowTimeout != null) clearTimeout(triggerEl.tooltipShowTimeout);
        triggerEl.tooltipShowTimeout = window.setTimeout(() => {
          triggerEl.tooltipInstance = createTooltip(triggerEl, binding.value, container);
        }, showDelay);
      } else if (triggerEl.tooltipInstance != null) {
        // some part of the tooltip is hovered, so cancel the hide timeout
        if (triggerEl.tooltipHideTimeout != null) clearTimeout(triggerEl.tooltipHideTimeout);
      }
    };
    triggerEl.tooltipOnMouseLeave = (e: MouseEvent) => {
      if (triggerEl.tooltipShowTimeout != null) clearTimeout(triggerEl.tooltipShowTimeout);
      triggerEl.tooltipHideTimeout = window.setTimeout(() => {
        destroyTooltip(triggerEl.tooltipInstance!);
      }, hideDelay);
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
    if (triggerEl.tooltipShowTimeout != null) {
      clearTimeout(triggerEl.tooltipShowTimeout);
    }
    if (triggerEl.tooltipHideTimeout != null) {
      clearTimeout(triggerEl.tooltipHideTimeout);
    }
    if (triggerEl.tooltipInstance != null) {
      _activeTooltips.value = _activeTooltips.value.filter((t) => t !== triggerEl.tooltipInstance);
    }
    if (triggerEl.tooltipOnMouseEnter) triggerEl.removeEventListener("mouseenter", triggerEl.tooltipOnMouseEnter);
    if (triggerEl.tooltipOnMouseLeave) triggerEl.removeEventListener("mouseleave", triggerEl.tooltipOnMouseLeave);
  },
};

//
// Hover directive
//

export type HoverInfo = {
  show?: (e: MouseEvent) => void;
  hide?: (e: MouseEvent) => void;
  showDelay?: number;
  hideDelay?: number;
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
        triggerEl.hoverShowTimeout = window.setTimeout(() => binding.value.show?.(e), showDelay);
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
  },
};
