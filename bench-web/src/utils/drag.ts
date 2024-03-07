import type { NodeReferenceData, NodeType } from "@/proto/wire";
import { toValueRef } from "@/utils/ref";
import { useEventListener, useMouseInElement } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export type DraggedType = NodeType | "native_file";
export type Dragged = Omit<NodeReferenceData, "metatype" | "type"> & {
  type: DraggedType;
};

export function setDragData(event: DragEvent, data: Dragged) {
  const json = JSON.stringify(data);
  event?.dataTransfer?.setData("application/symbolx.bench." + data.type, json);
}

export function useRelativeDropZone(
  target: Ref<HTMLElement | null | undefined>,
  types: DraggedType[],
  onDrop?: (thing: File[] | Dragged | null) => void,
  enabled?: Ref<boolean>,
) {
  enabled = enabled ?? ref(true);
  const isOverDropZone = ref(false);
  const position = useMouseInElement(target);
  let counter = 0;

  function getType(event: DragEvent): DraggedType | null {
    // either files or one of our types
    if (event.dataTransfer?.types != null) {
      for (const type of event.dataTransfer.types) {
        if (type.startsWith("application/symbolx.bench.")) {
          const subtype = type.slice("application/symbolx.bench.".length);
          return (subtype.charAt(0).toUpperCase() + subtype.slice(1)) as any;
        }
      }
      if (event.dataTransfer.types.includes("Files")) {
        return "native_file";
      }
    }
    return null;
  }

  function isDropAllowed(event: DragEvent): boolean {
    const type = getType(event);
    if (type == null) {
      return false;
    }
    return types.includes(type);
  }

  useEventListener<DragEvent>(target, "dragenter", (event) => {
    if (!isDropAllowed(event)) {
      return;
    }
    event.preventDefault();
    counter += 1;
    isOverDropZone.value = true;
  });
  useEventListener<DragEvent>(target, "dragover", (event) => {
    if (!isDropAllowed(event)) {
      return;
    }
    event.preventDefault();
  });
  useEventListener<DragEvent>(target, "dragleave", (event) => {
    if (!isDropAllowed(event)) {
      return;
    }
    event.preventDefault();
    counter -= 1;
    if (counter <= 0) isOverDropZone.value = false;
  });
  useEventListener<DragEvent>(target, "drop", (event) => {
    if (!isDropAllowed(event)) {
      return;
    }
    isOverDropZone.value = false;
    if (!enabled?.value) {
      return;
    }
    event.preventDefault();
    counter = 0;
    const type = getType(event);
    if (type == "native_file") {
      const files = Array.from(event.dataTransfer?.files ?? []);
      onDrop?.(files.length === 0 ? null : files);
    } else if (type != null) {
      const data = event.dataTransfer?.getData("application/symbolx.bench." + type);
      onDrop?.(data == null ? null : JSON.parse(data));
    }
  });

  const inTopHalf = computed(() => position.elementY.value < position.elementHeight.value / 2);
  const inBottomHalf = computed(() => position.elementY.value > position.elementHeight.value / 2);
  const inLeftHalf = computed(() => position.elementX.value < position.elementWidth.value / 2);
  const inRightHalf = computed(() => position.elementX.value > position.elementWidth.value / 2);

  return {
    isOverDropZone: toValueRef(computed(() => enabled?.value && isOverDropZone.value)),
    position,
    inTopHalf,
    inBottomHalf,
    inLeftHalf,
    inRightHalf,
  };
}
