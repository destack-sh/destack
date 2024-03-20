import type { IconData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { isOnMac } from "@/utils/browser";
import { normalizeKeymapKey, parseKeymapSignature } from "@/utils/keymap";
import { Casing, toCasing } from "@/utils/string";
import { computed, type FunctionalComponent } from "vue";

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
export const Shortcut: FunctionalComponent<{ shortcut: string }> = (props, context) => {
  const parsed = computed(() => parseKeymapSignature(props.shortcut));
  return (
    // Shortcut
    <span class="flex flex-row gap-x-2">
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

// TODO :UI: position & animate tooltips better
// TODO :UI :Performance: create (and destroy) tooltip element on the fly
export const Tooltip: FunctionalComponent<{
  icon?: string | IconData;
  title?: string;
  text: string;
  shortcut?: string;
  position: string;
}> = (props, context) => {
  const icon = typeof props.icon === "string" ? makeIcon({ name: props.icon }) : props.icon;
  const element = (
    <div
      class={
        props.position +
        " pointer-events-none absolute z-30 min-w-fit max-w-60 whitespace-nowrap rounded-md border border-gray-300 bg-white px-2.5 py-1 text-left text-gray-700 opacity-0 shadow-md shadow-gray-300 transition-opacity group-hover:opacity-100"
      }
    >
      {props.icon || props.title ? (
        <p class="mb-0.5 flex flex-row items-center gap-x-1.5">
          {icon?.name ? <i class={`text-gray-600 ${icon.name}`} /> : null}
          {props.title ? <h3 class="font-semibold">{props.title}</h3> : null}
          <span class="ml-auto">{props.shortcut ? <Shortcut shortcut={props.shortcut} /> : null}</span>
        </p>
      ) : null}
      <p>{props.text}</p>
    </div>
  );
  return element;
};
