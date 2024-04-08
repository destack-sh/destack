import { NodeType, type NodeReferenceData, SelectionData, Orientation } from "@/proto/wire";
import { useEventListener, useMouse, useMouseInElement } from "@vueuse/core";
import { computed, ref, type MaybeRef, type Ref, toRef } from "vue";

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
      kind: "file"; // native browser file
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

/**
 * Track certain drop events in a target region.
 */
export function useDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged) => void;
  enabled?: Ref<boolean>;
}): { isInDropZone: Ref<boolean> } {
  const enabled = options.enabled ?? ref(true);
  const isOverDropZone = ref(false);
  let counter = 0;

  function getDraggedMeta(event: DragEvent): { kind: DraggedKind; metatypes: NodeType[] } | null {
    // get dragged metatype while dragging (can only read keys set in setDragData)
    if (event.dataTransfer?.types != null) {
      // extract with string matching
      const kind: DraggedKind | undefined = options.kinds.find((k) =>
        event.dataTransfer?.types.includes("application/symbolx.bench." + k),
      );
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
    isInDropZone: computed(() => enabled?.value && isOverDropZone.value),
  };
}

/**
 * Track certain drop zone events in a single target region.
 */
export function useSingleDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  orientation: MaybeRef<Orientation>;
  onDrop?: (dragged: Dragged, anchor: "start" | "end") => void;
  enabled?: Ref<boolean>;
}): { activeDropZone: Ref<{ anchor: "start" | "end" } | null>; getActiveDropZone: () => { anchor: "start" | "end" } } {
  const { isInDropZone } = useDropZone({
    ...options,
    onDrop: (dragged) => {
      options.onDrop?.(dragged, getActiveDropZone().anchor);
    },
  });

  function getActiveDropZone(): { anchor: "start" | "end" } {
    if (orientation.value == Orientation.HORIZONTAL)
      return { anchor: position.elementX.value < position.elementWidth.value / 2 ? "start" : "end" };
    else return { anchor: position.elementY.value < position.elementHeight.value / 2 ? "start" : "end" };
  }

  const orientation = toRef(options.orientation) as Ref<Orientation>;
  const position = useMouseInElement(options.container);
  const activeDropZone: Ref<{ anchor: "start" | "end" } | null> = computed(() =>
    isInDropZone.value ? getActiveDropZone() : null,
  );
  return { activeDropZone, getActiveDropZone };
}

export type SplitAnchor = "center" | "left" | "top" | "right" | "bottom";
export const SPLIT_EDGE_ZONE_FRACTION = 0.12;

/*
 * Track 'split' container drop events.
 * The left/top/right/bottom fraction percent are the respective zones, the rest is the center zone.
 * If the cursor is in two zones at once, the edge we're closest to wins.
 */
export function useSplitDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged, anchor: SplitAnchor) => void;
  enabled?: Ref<boolean>;
}): {
  activeDropZone: Ref<{ anchor: SplitAnchor; splitClass: string } | null>;
} {
  const { isInDropZone } = useDropZone({
    ...options,
    onDrop: (dragged) => {
      options.onDrop?.(dragged, getActiveDropZone().anchor);
    },
  });

  const position = useMouseInElement(options.container);

  function getActiveDropZone(): { anchor: SplitAnchor; splitClass: string } {
    const mouseX = position.elementX.value;
    const mouseY = position.elementY.value;
    const distances = {
      left: mouseX,
      top: mouseY,
      right: position.elementWidth.value - mouseX,
      bottom: position.elementHeight.value - mouseY,
    };
    const closestEdge = Object.keys(distances).reduce((a, b) =>
      (distances as any)[a] < (distances as any)[b] ? a : b,
    );

    const horizontalEdgeZone = position.elementWidth.value * SPLIT_EDGE_ZONE_FRACTION;
    const verticalEdgeZone = position.elementHeight.value * SPLIT_EDGE_ZONE_FRACTION;
    let anchor: SplitAnchor;
    switch (closestEdge) {
      case "left":
        anchor = mouseX <= horizontalEdgeZone ? "left" : "center";
        break;
      case "top":
        anchor = mouseY <= verticalEdgeZone ? "top" : "center";
        break;
      case "right":
        anchor = mouseX >= position.elementWidth.value - horizontalEdgeZone ? "right" : "center";
        break;
      case "bottom":
        anchor = mouseY >= position.elementHeight.value - verticalEdgeZone ? "bottom" : "center";
        break;
      default:
        anchor = "center";
    }

    const splitClass = {
      top: "left-0 top-0 w-full h-1/2",
      bottom: "left-0 top-1/2 w-full h-1/2",
      left: "left-0 top-0 w-1/2 h-full",
      right: "left-1/2 top-0 w-1/2 h-full",
      center: "left-0 top-0 w-full h-full",
    }[anchor];

    return { anchor, splitClass };
  }

  const activeDropZone: Ref<{ anchor: SplitAnchor; splitClass: string } | null> = computed(() =>
    isInDropZone.value ? getActiveDropZone() : null,
  );
  return { activeDropZone };
}

/**
 * Track certain drop zone events across dynamic target regions in a single parent container.
 */
export function useMultiDropZone(options: {
  container: Ref<HTMLElement | null | undefined>;
  targets: Ref<Record<string, HTMLElement>>;
  orientation: MaybeRef<Orientation>;
  kinds: DraggedKind[];
  metatypes: NodeType[];
  onDrop?: (dragged: Dragged, anchor: "start" | "end", targetId: string | null) => void;
}): { activeDropZone: Ref<{ anchor: "start" | "end"; targetId: string | null } | null> } {
  const { activeDropZone: singleDropZone, getActiveDropZone: getSingleActiveDropZone } = useSingleDropZone({
    container: options.container,
    kinds: options.kinds,
    metatypes: options.metatypes,
    orientation: options.orientation,
    onDrop: (dragged) => {
      const { anchor, targetId } = getActiveDropZone()!;
      options.onDrop?.(dragged, anchor, targetId);
    },
  });

  function getActiveDropZone(): { anchor: "start" | "end"; targetId: string | null } {
    // find directly hit zone
    const cursor = { x: position.x.value, y: position.y.value };
    for (const [targetId, targetEl] of Object.entries(options.targets.value)) {
      const targetRect = targetEl.getBoundingClientRect();
      const isOver =
        options.orientation == Orientation.HORIZONTAL
          ? cursor.x > targetRect.left && cursor.x < targetRect.right
          : cursor.y > targetRect.top && cursor.y < targetRect.bottom;
      if (isOver) {
        const anchor =
          options.orientation == Orientation.HORIZONTAL
            ? cursor.x < targetRect.left + targetRect.width / 2
              ? "start"
              : "end"
            : cursor.y < targetRect.top + targetRect.height / 2
              ? "start"
              : "end";
        return { targetId, anchor };
      }
    }

    // otherwise if we have targets we attribute to first/last target (sorted by position)
    const singleDropZone = getSingleActiveDropZone();
    const targetsSorted = Object.entries(options.targets.value).sort(([targetId, targetEl]) => {
      const targetRect = targetEl.getBoundingClientRect();
      return options.orientation == Orientation.HORIZONTAL ? targetRect.left : targetRect.top;
    });
    if (targetsSorted.length > 0) {
      // anchor=end assumes that targets are positioned start to end in the container
      if (singleDropZone.anchor == "start") {
        return { targetId: targetsSorted[0][0], anchor: "end"};
      } else {
        return { targetId: targetsSorted[targetsSorted.length - 1][0], anchor: "end" };
      }
    }

    // else we attribute to entire container
    return { targetId: null, anchor: singleDropZone.anchor };
  }

  const position = useMouse();
  const activeDropZone: Ref<{ anchor: "start" | "end"; targetId: string | null } | null> = computed(() =>
    singleDropZone.value != null ? getActiveDropZone() : null,
  );

  return { activeDropZone };
}
