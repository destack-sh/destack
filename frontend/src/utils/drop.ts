import { useEventListener, useMouseInElement } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export function useRelativeDropZone(
  target: Ref<HTMLElement | null | undefined>,
  onDrop?: (files: File[] | null) => void,
  enabled?: Ref<boolean>
) {
  enabled = enabled ?? ref(true);
  const isOverDropZone = ref(false);
  const position = useMouseInElement(target);
  let counter = 0;

  useEventListener<DragEvent>(target, "dragenter", (event) => {
    event.preventDefault();
    counter += 1;
    isOverDropZone.value = true;
  });
  useEventListener<DragEvent>(target, "dragover", (event) => {
    event.preventDefault();
  });
  useEventListener<DragEvent>(target, "dragleave", (event) => {
    event.preventDefault();
    counter -= 1;
    if (counter === 0) isOverDropZone.value = false;
  });
  useEventListener<DragEvent>(target, "drop", (event) => {
    if (!enabled?.value) {
      return;
    }
    event.preventDefault();
    counter = 0;
    isOverDropZone.value = false;
    const files = Array.from(event.dataTransfer?.files ?? []);
    onDrop?.(files.length === 0 ? null : files);
  });

  const inTopHalf = computed(() => position.elementY.value < position.elementHeight.value / 2);
  const inBottomHalf = computed(() => position.elementY.value > position.elementHeight.value / 2);

  return {
    isOverDropZone: computed(() => enabled?.value && isOverDropZone.value),
    position,
    inTopHalf,
    inBottomHalf,
  };
}
