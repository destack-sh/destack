import { NodeType, type NodeReferenceData, SelectionData } from "@/proto/wire";
import { useEventListener, useMouseInElement } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export type DraggedKind = "node" | "selection" | "file";
export type Dragged =
  | {
      kind: "node";
      node: NodeReferenceData;
    }
  | {
      kind: "selection";
      selection: SelectionData;
    }
  | {
      kind: "file";
      files: File[];
    };

export function setDragData(event: DragEvent, data: Dragged) {
  // we can't read the value of dataTransfer while dragging so we need all the info in the keys
  const dt = event.dataTransfer;
  if (!dt) throw new Error(`no dataTransfer on event: ${event}`);
  dt.setData("application/symbolx.bench." + data.kind, JSON.stringify(data));

  let metatypes: NodeType[] = [];
  if (data.kind == "node") {
    metatypes = [data.node.type];
  } else if (data.kind == "selection") {
    metatypes = [
      ...(data.selection.nodesPtr?.map((ptr) => ptr.type) ?? []),
      data.selection.fromNodePtr?.type,
      data.selection.toNodePtr?.type,
    ]
      .filter((t) => t != null)
      .map((t) => t!);
  }
  metatypes.forEach((t) => dt.setData("application/symbolx.bench.metatypes." + t, t.toString()));
}

export function useDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged) => void;
  enabled?: Ref<boolean>;
}) {
  const enabled = options.enabled ?? ref(true);
  const isOverDropZone = ref(false);
  let counter = 0;

  function getDraggedMeta(event: DragEvent): { kind: DraggedKind; metatypes: NodeType[] } | null {
    // get dragged metatype while dragging (can only read keys set in setDragData)
    if (event.dataTransfer?.types != null) {
      // extract with string matching
      const kind: DraggedKind | undefined = options.kinds.find((k) => event.dataTransfer?.types.includes("application/symbolx.bench." + k));
      if (kind != null) {
        const metatypes: NodeType[] = [];
        for (const type of event.dataTransfer.types) {
          if (type.startsWith("application/symbolx.bench.metatypes.")) {
            const metatype = type.slice("application/symbolx.bench.metatypes.".length);
            metatypes.push(Number(metatype) as NodeType);
          }
        }
        return { kind, metatypes };
      }
    }
    return null;
  }

  function isDropAllowed(event: DragEvent): boolean {
    const meta = getDraggedMeta(event);
    if (meta == null) return false;
    else return options.kinds.includes(meta.kind) && !meta.metatypes.some((t) => !options.metatypes.includes(t));
  }

  useEventListener<DragEvent>(options.container, "dragenter", (event) => {
    if (!isDropAllowed(event)) return;
    event.preventDefault();
    counter += 1;
    isOverDropZone.value = true;
  });
  useEventListener<DragEvent>(options.container, "dragover", (event) => {
    if (!isDropAllowed(event)) return;
    event.preventDefault();
  });
  useEventListener<DragEvent>(options.container, "dragleave", (event) => {
    if (!isDropAllowed(event)) return;
    event.preventDefault();
    counter -= 1;
    if (counter <= 0) isOverDropZone.value = false;
  });
  useEventListener<DragEvent>(options.container, "drop", (event) => {
    if (!isDropAllowed(event)) return;
    isOverDropZone.value = false;
    if (!enabled?.value) return;
    event.preventDefault();
    counter = 0;
    const meta = getDraggedMeta(event);
    if (meta?.kind == "file") {
      const files = Array.from(event.dataTransfer?.files ?? []);
      options.onDrop?.({ kind: "file", files });
    } else if (meta != null) {
      const data = event.dataTransfer?.getData("application/symbolx.bench." + meta.kind);
      if (data != null) {
        options.onDrop?.(JSON.parse(data));
      }
    }
  });

  return {
    isOverDropZone: computed(() => enabled?.value && isOverDropZone.value),
  };
}

export function useSingleDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged) => void;
  enabled?: Ref<boolean>;
}) {
  const { isOverDropZone } = useDropZone(options);

  const position = useMouseInElement(options.container);
  const inTopHalf = computed(() => position.elementY.value < position.elementHeight.value / 2);
  const inBottomHalf = computed(() => position.elementY.value > position.elementHeight.value / 2);
  const inLeftHalf = computed(() => position.elementX.value < position.elementWidth.value / 2);
  const inRightHalf = computed(() => position.elementX.value > position.elementWidth.value / 2);

  return { isOverDropZone, inTopHalf, inBottomHalf, inLeftHalf, inRightHalf };
}

export function useMultiDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  targets: Ref<Record<string, HTMLElement>>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged, targetId: string) => void;
}) {
  throw new Error("not implemented");
}
