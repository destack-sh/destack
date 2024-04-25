// TODO :Architecture: figure out proper all-encompassing event system/bus

import { Orientation, Variant, ViewData, ViewType, type NodeReferenceData, type NodeType } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/system/action";
import { Casing, toCasing } from "@/utils/string";
import { v4 } from "uuid";
import { computed, getCurrentInstance, type ComponentInstance, type FunctionalComponent, type Ref } from "vue";

export type ViewProps = { self?: NodeReferenceData; modelValue?: any } & Partial<
  Omit<ViewData, "metatype" | "id" | "ck" | "revision" | "setProperties">
>;
export type ViewComponent = {
  new (): ComponentInstance<any>;
  props: ViewProps;
  exposed: ViewExposed;
};

export const VIEW_EMITS = {
  apply: null,
  cancel: null,
  ["update:modelValue"]: null,
};

export function viewEmits(): Partial<typeof VIEW_EMITS> {
  return VIEW_EMITS;
}

export type FocusAnchor = "left" | "right" | "top" | "bottom" | "center";

export type ViewExposed = (
  | {
      // always identity
      /** The view node identity of a view component */
      self: Ref<TypedNodeReferenceData<NodeType.VIEW>>;
      id?: Ref<string>;
    }
  | {
      // maybe anonymous identity
      /** The view node identity of a view component, maybe null if anonymous */
      self: Ref<TypedNodeReferenceData<NodeType.VIEW> | null | undefined>;
      /** The anonymous identity of a view component if 'self' is unavailable.  */
      id: Ref<string>;
    }
) & {
  /** The virtual actions implemented by this view */
  actions?: Partial<ActionMapImplementation<any>>;
  /** The supported variants (if any) */
  variants?: Variant[];

  /** Focus the element at the given anchor inside the view OR return the element to focus. May be a view or any element. */
  focus?: (anchor: FocusAnchor | NodeReferenceData) => void | boolean | ViewComponent | HTMLElement | null;
  /** Map the relevant node at the given element. */
  mapToNode?: (element: HTMLElement | ViewComponent) => NodeReferenceData | null;
} & {};

// TODO :Architecture :Performance: revisit content view wrapper for vapor mode
export const ViewContentWrapper: FunctionalComponent<{
  title?: string;
  variant?: Variant;
  orientation?: Orientation;
}> = (props, { slots }) => {
  const classBase =
    props.title == null
      ? ""
      : props.orientation === Orientation.HORIZONTAL
        ? "flex flex-row items-center justify-between gap-x-5"
        : "flex flex-col";

  const labelClass =
    props.variant === Variant.PRIMARY ? "mb-0.5 block font-semibold text-gray-900" : "mb-0.5 block text-gray-700";

  return (
    <div class={classBase}>
      {props.title && <label class={labelClass}>{props.title}</label>}
      {slots.default ? slots.default() : null}
    </div>
  );
};
ViewContentWrapper.props = ["title", "variant", "orientation"];

/** Creates an id for an 'anonymous' view */
function deriveViewId(selfId: string, name: string): string {
  return `${selfId}.${name}`;
}

/** Creates a dynamic view id ref for View components that sometimes don't have a 'self' node identity */
export function makeViewId(props: { self?: NodeReferenceData; name?: string | null }): Ref<string> {
  const instance = getCurrentInstance()!;
  if (instance == null) throw new Error("no Vue instance");
  return computed(() => {
    if (props.self?.id != null) return props.self.id;

    const instanceInternalId = instance.uid;
    let parent = instance.parent;
    while (parent != null) {
      const exposed = (parent as unknown as ViewComponent).exposed;
      if (exposed?.self?.value?.id != null)
        return deriveViewId(exposed.self.value.id, props.name ?? instanceInternalId.toString());
      else if (exposed?.id?.value != null)
        return deriveViewId(exposed.id.value, props.name ?? instanceInternalId.toString());
      parent = parent.parent;
    }
    // this is a component outside of parent view, just use a random id
    return v4();
  });
}

// inverse :ViewRegistry for lookups without needing to import the registry
export function getViewTypeByComponentName(name: string): ViewType | null {
  if (name == "HtmlInput") {
    return ViewType.STRING;
  } else {
    const capsName = toCasing(name, Casing.ALL_CAPS);
    return (ViewType[capsName as any] as unknown as ViewType) ?? null;
  }
}
