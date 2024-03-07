import { type FunctionalComponent } from "vue";

export function shortcut(shorcut: string) {
  return (
    <span class="rounded-md border border-gray-400 bg-primary-300 px-1 text-xs font-semibold uppercase text-gray-600 shadow-sm shadow-gray-200">
      {shorcut}
    </span>
  );
}

// TODO :UI: position & animate tooltips better
export const Tooltip: FunctionalComponent<{
  icon?: string;
  title?: string;
  text: string;
  shortcut?: string;
  position: string;
}> = (props, context) => {
  const element = (
    <div
      class={
        props.position + " pointer-events-none absolute z-30 min-w-fit max-w-60 whitespace-nowrap rounded-md border border-gray-300 bg-gray-100 px-2.5 py-1 text-left text-gray-700 opacity-0 shadow-sm shadow-gray-300 transition-opacity group-hover:opacity-100"
      }
    >
      {props.icon || props.title ? (
        <p class="mb-0.5 flex flex-row items-center gap-x-1.5">
          {props.icon ? <i class={`text-gray-600 ${props.icon}`}></i> : null}
          {props.title ? <h3 class="font-semibold">{props.title}</h3> : null}
          {props.shortcut ? shortcut(props.shortcut) : null}
        </p>
      ) : null}
      <p>{props.text}</p>
    </div>
  );
  return element;
};
