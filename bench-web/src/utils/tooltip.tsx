import type { IconData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { parseKeymapSignature, renderKeymapKey } from "@/utils/keymap";
import { Casing, toCasing } from "@/utils/string";
import { computed, type FunctionalComponent } from "vue";

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
              <kbd class="rounded-md border border-gray-300 bg-white px-1.5 py-0.5 text-xs text-gray-900">
                {toCasing(renderKeymapKey(key), Casing.CAMEL)}
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
        " pointer-events-none absolute z-30 min-w-fit max-w-60 whitespace-nowrap rounded-md border border-gray-300 bg-gray-100 px-2.5 py-1 text-left text-gray-700 opacity-0 shadow-sm shadow-gray-300 transition-opacity group-hover:opacity-100"
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
