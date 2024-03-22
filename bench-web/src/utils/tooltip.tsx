import type { IconData, LogLevel, TextData } from "@/proto/wire";
import type { Action } from "@/system/action";
import { makeIcon, toIconMaybe } from "@/system/icon";
import { isOnMac } from "@/utils/browser";
import { findFloatingContainer, type FloatingOptions, type FloatingPlacement } from "@/utils/floating";
import { normalizeKeymapKey, parseKeymapSignature } from "@/utils/keymap";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import type { MaybeElement } from "@vueuse/core";
import { computed, type FunctionalComponent, type Directive, type Component, shallowRef, type Ref } from "vue";

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
              <kbd class="min-w-5 rounded-md border border-gray-300 bg-white px-1 py-0.5 text-center font-sans text-xs text-gray-700 hover:border-orange-900 hover:bg-gray-100 hover:text-orange-900">
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

const DEFAULT_SHOW_DELAY = 500;
const DEFAULT_HIDE_DELAY = 300;

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
    title: action.title,
    text: action.text,
    shortcuts: action.shortcuts,
    arrow: true,
    placement: "top",
  };
}

/** Store state on the triggering element */
interface TriggerElement extends HTMLElement {
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
  reference: TriggerElement;
  container?: HTMLElement | SVGElement;
};

let tooltipId = 0;
function createTooltipInstance(
  reference: TriggerElement,
  container: HTMLElement | SVGElement | undefined,
  info: TooltipInfo,
): TooltipInstance {
  const instance = { id: tooltipId++, info, reference, container };
  activeTooltips.value = [...activeTooltips.value, instance];
  return instance;
}

function destroyTooltipInstance(instance: TooltipInstance) {
  activeTooltips.value = activeTooltips.value.filter((t) => t !== instance);
}

export const activeTooltips: Ref<TooltipInstance[]> = shallowRef([]);

/** Simple tooltip directive that shows/hides itself on hover with a delay*/
export const TOOLTIP_DIRECTIVE: Directive<MaybeElement, TooltipInfo> = {
  mounted(el, binding) {
    const triggerEl = el as TriggerElement;
    const showDelay = binding.value.showDelay ?? DEFAULT_SHOW_DELAY;
    const hideDelay = binding.value.hideDelay ?? DEFAULT_HIDE_DELAY;

    triggerEl.tooltipOnMouseEnter = (e: MouseEvent) => {
      if (e.target == triggerEl) {
        // create a new tooltip instance if trigger is hovered for a while
        const container = findFloatingContainer(triggerEl) ?? undefined;
        if (triggerEl.tooltipShowTimeout != null) clearTimeout(triggerEl.tooltipShowTimeout);
        triggerEl.tooltipShowTimeout = window.setTimeout(() => {
          triggerEl.tooltipInstance = createTooltipInstance(triggerEl, container, binding.value);
        }, showDelay);
      } else if (triggerEl.tooltipInstance != null) {
        // some part of the tooltip is hovered, so cancel the hide timeout
        if (triggerEl.tooltipHideTimeout != null) clearTimeout(triggerEl.tooltipHideTimeout);
      }
    };
    triggerEl.tooltipOnMouseLeave = (e: MouseEvent) => {
      if (triggerEl.tooltipShowTimeout != null) clearTimeout(triggerEl.tooltipShowTimeout);
      triggerEl.tooltipHideTimeout = window.setTimeout(() => {
        destroyTooltipInstance(triggerEl.tooltipInstance!);
      }, hideDelay);
    };
    triggerEl.addEventListener("mouseenter", triggerEl.tooltipOnMouseEnter);
    triggerEl.addEventListener("mouseleave", triggerEl.tooltipOnMouseLeave);
  },

  updated(el, binding) {
    const tooltipEl = el as TriggerElement;
    if (tooltipEl.tooltipInstance != null) {
      tooltipEl.tooltipInstance.info = binding.value;
    }
  },

  unmounted(el) {
    const tooltipEl = el as TriggerElement;
    if (tooltipEl.tooltipShowTimeout != null) {
      clearTimeout(tooltipEl.tooltipShowTimeout);
    }
    if (tooltipEl.tooltipHideTimeout != null) {
      clearTimeout(tooltipEl.tooltipHideTimeout);
    }
    if (tooltipEl.tooltipInstance != null) {
      activeTooltips.value = activeTooltips.value.filter((t) => t !== tooltipEl.tooltipInstance);
    }
  },
};
