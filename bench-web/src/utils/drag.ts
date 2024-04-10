import {
  BenchType,
  NodeType,
  Orientation,
  SelectionData,
  type AnyNodeData,
  type NodeReferenceData,
} from "@/proto/wire";
import { toNodeReference, type AnyNodeReferenceData } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import { getElement, getElementRef } from "@/utils/element";
import { log } from "@/utils/log";
import { uuidt } from "@/utils/uuidt";
import { tryOnBeforeUnmount, useEventListener, useMouse, useMouseInElement, type MaybeElement } from "@vueuse/core";
import type { AnyNode } from "postcss";
import { computed, ref, shallowRef, toRef, unref, watch, type MaybeRef, type Ref } from "vue";

// NOTE: for now the drag/drop system is only expected to work within a single bench-web instance
//  (i.e. not across instances, but it should work across with other applications)

const DRAGGED_KINDS = ["node", "selection", "file"] as const;
export type DraggedKind = (typeof DRAGGED_KINDS)[number];
export type DraggedData =
  | {
      kind: "node";
      node: NodeReferenceData;
      nodes: AnyNodeData[];
    }
  | {
      kind: "selection";
      selection: SelectionData;
      nodes: AnyNodeData[];
    }
  | {
      kind: "file"; // native browser file
      fileTypes: string[];
      files?: File[];
    };
export type Dragged = {
  id: string;
} & DraggedData;

// NOTE: we render the drag image globally in Space
const dragImageRef = ref<HTMLElement | null>(null);

export function _setDragImage(image: HTMLElement | null) {
  dragImageRef.value = image;
}

export const activeDragged: Ref<Dragged | null> = shallowRef(null);
const dropZones: Ref<Record<number, DropZone>> = shallowRef({});
const dropZonesByElement: Map<HTMLElement | SVGElement, DropZone> = new Map();
const activeDropZone: Ref<DropZone | null> = ref(null);

/** Start dragging the given thing. Sets 'activeDragged'. */
export function startDragging(
  event: DragEvent,
  graph: ReadNodeGraph,
  data: AnyNodeData | AnyNodeReferenceData | SelectionData | DraggedData,
) {
  let dragged: Dragged;
  if ("metatype" in data) {
    if (data.metatype == BenchType.NODE_REFERENCE) {
      dragged = {
        id: uuidt(),
        kind: "node",
        node: data as NodeReferenceData,
        nodes: [graph.getOrFail(data as NodeReferenceData)],
      };
    } else if (data.metatype == BenchType.SELECTION) {
      dragged = {
        id: uuidt(),
        kind: "selection",
        selection: data as SelectionData,
        nodes: (data as SelectionData).nodesPtr.map((n) => graph.getOrFail(n)),
      };
    } else {
      dragged = {
        id: uuidt(),
        kind: "node",
        node: toNodeReference(data as AnyNodeData),
        nodes: [data as AnyNodeData],
      };
    }
  } /* DraggedData */ else {
    dragged = { id: uuidt(), ...data };
  }

  const dt = event.dataTransfer;
  if (!dt) throw new Error(`no dataTransfer on event: ${event}`);
  dt.setData("application/symbolx.bench." + dragged.id, JSON.stringify({ ...dragged, nodes: [] }));
  dt.setDragImage(dragImageRef.value!, -10, 0);

  if (activeDragged.value != null) log.warn("drag.alreadyExists", activeDragged);
  activeDragged.value = dragged;
  log.debug("drag.start", dragged);
}

/** Gets the current dragged thing. Must match 'activeDragged'. */
function getDraggedData(event: DragEvent): DraggedData | null {
  // get dragged metatype while dragging (can only read keys set in startDragging above)
  if (event.dataTransfer?.types == null) return null;

  // check if it's one of our dragged items
  const draggedId = event.dataTransfer.types
    .find((t) => t.startsWith("application/symbolx.bench."))
    ?.split(".")
    .pop();
  if (draggedId != null) {
    if (activeDragged.value?.id == draggedId) return activeDragged.value;
    else log.warn("drag.notFound", draggedId, activeDragged);
  }

  // might be a file drop
  if (event.dataTransfer.types.includes("Files")) {
    // get files if available
    if (event.dataTransfer.files.length > 0) {
      return {
        kind: "file",
        fileTypes: Array.from(event.dataTransfer.types),
        files: Array.from(event.dataTransfer.files),
      };
    }
  }

  // something else
  return null;
}

/** General options for any drop zone. */
type DropOptions = {
  /** Name of the drop zone for debugging */
  name: string;
  /** The top level container for the zone. */
  container: Ref<MaybeElement>;
  /** The kinds of supported drag kinds. */
  kinds?: MaybeRef<DraggedKind[]>;
  /** The metatypes of supported drag nodes (for Dragged with nodes). */
  metatypes?: MaybeRef<NodeType[]>;
  /** The allowed file types for file drops. */
  fileTypes?: MaybeRef<string[]>;
  /** Whether the drop zone is enabled. */
  enabled?: Ref<boolean>;
};

//
// Drop zones
// NOTE: to handl hierarchical drop zones, we manage them globally here and query against thee dropZones registry.
//

type DropZone = DropOptions & {
  id: number;
  containerEl: Ref<HTMLElement | SVGElement | null>;
  onDrop?: (dragged: DraggedData) => void;
};
let dropZoneId = 0;
function newDropZoneId(): number {
  return dropZoneId++;
}

/** Whether dropping the dragged thing into this zone is possible */
function isDropCompatible(zone: DropZone, dragged: DraggedData): boolean {
  if (dragged == null) return false;
  const kinds = unref(zone.kinds);
  if (kinds != null && !kinds.includes(dragged.kind)) return false;
  const metatypes = unref(zone.metatypes);
  if (metatypes != null && dragged.kind == "node" && !metatypes.includes(dragged.node.type)) return false;
  const fileTypes = unref(zone.fileTypes);
  if (fileTypes != null && dragged.kind == "file" && dragged.fileTypes.some((t) => !fileTypes.includes(t)))
    return false;
  return true;
}

/** Traverses the event targets up to find a */
function findCompatibleDropZone(el: HTMLElement | SVGElement | null, dragged: DraggedData): DropZone | null {
  while (el) {
    const zone = dropZonesByElement.get(el);
    if (zone && isDropCompatible(zone, dragged)) return zone;
    el = el.parentElement;
  }
  return null;
}

function updateDropZone(event: DragEvent) {
  const dragged = getDraggedData(event);
  if (dragged == null) return;
  event.preventDefault();
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone?.id !== activeDropZone.value?.id) {
    activeDropZone.value = zone;
    log.trace("drag.activeZone", zone);
  }
}

const resetDrag = () => {
  activeDragged.value = null;
  activeDropZone.value = null;
};

useEventListener("dragenter", updateDropZone);
useEventListener("dragover", updateDropZone);
useEventListener("drop", (event) => {
  const dragged = getDraggedData(event);
  if (dragged == null) return;
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone) {
    log.debug("drag.drop", dragged, zone);
    zone.onDrop?.(dragged);
  }
  resetDrag();
});
useEventListener("dragend", resetDrag);

/**
 * Track certain drop events in a target region.
 */
export function useDropZone(
  options: DropOptions & {
    onDrop?: (dragged: DraggedData) => void;
  },
): { isInDropZone: Ref<boolean> } {
  const enabled = options.enabled ?? ref(true);

  // create & register/deregister zone
  const zone = {
    id: newDropZoneId(),
    containerEl: getElementRef(options.container),
    onDrop: options.onDrop,
    ...options,
  };
  dropZones.value[zone.id] = zone;
  watch(
    zone.containerEl,
    (newEl, oldEl) => {
      if (oldEl) dropZonesByElement.delete(oldEl);
      if (newEl) dropZonesByElement.set(newEl, zone);
    },
    { immediate: true },
  );
  tryOnBeforeUnmount(() => {
    delete dropZones.value[zone.id];
    if (zone.containerEl.value) dropZonesByElement.delete(zone.containerEl.value);
  });

  return {
    isInDropZone: computed(() => enabled.value && activeDropZone.value?.id == zone.id),
  };
}

/**
 * Track certain drop zone events in a single target region.
 */
export function useSingleDropZone(
  options: DropOptions & {
    orientation: MaybeRef<Orientation>;
    onDrop?: (dragged: DraggedData, anchor: "start" | "end") => void;
  },
): { activeDropZone: Ref<{ anchor: "start" | "end" } | null>; getActiveDropZone: () => { anchor: "start" | "end" } } {
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
export function useSplitDropZone(
  options: DropOptions & {
    onDrop?: (dragged: DraggedData, anchor: SplitAnchor) => void;
  },
): {
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

type MultiAnchor = "start" | "center" | "end";
/**
 * Track certain drop zone events across dynamic target regions in a single parent container.
 */
export function useMultiDropZone(
  options: DropOptions & {
    targets: Ref<Record<string, MaybeElement>>;
    orientation: MaybeRef<Orientation>;
    defaultToEdge?: boolean;
    hasCenterAnchor?: boolean;
    allowDrop?: (dragged: DraggedData, anchor: MultiAnchor, targetId: string) => boolean;
    onDrop?: (dragged: DraggedData, anchor: MultiAnchor, targetId: string | null) => void;
  },
): { activeDropZone: Ref<{ anchor: MultiAnchor; targetId: string | null } | null> } {
  const { activeDropZone: singleDropZone, getActiveDropZone: getSingleActiveDropZone } = useSingleDropZone({
    ...options,
    orientation: options.orientation,
    onDrop: (dragged) => {
      const { anchor, targetId } = getActiveDropZone()!;
      if (targetId == null || options.allowDrop?.(dragged, anchor, targetId) !== false) {
        options.onDrop?.(dragged, anchor, targetId);
      }
    },
  });

  function getActiveDropZone(): { anchor: MultiAnchor; targetId: string | null } {
    // find directly hit zone
    const cursor = { x: position.x.value, y: position.y.value };
    for (const [targetId, targetEl] of Object.entries(options.targets.value)) {
      const targetRect = getElement(targetEl)!.getBoundingClientRect();
      const cursorP = options.orientation == Orientation.HORIZONTAL ? cursor.x : cursor.y;
      const targetStart = options.orientation == Orientation.HORIZONTAL ? targetRect.left : targetRect.top;
      const targetEnd = options.orientation == Orientation.HORIZONTAL ? targetRect.right : targetRect.bottom;
      if (cursorP >= targetStart && cursorP <= targetEnd) {
        // if hasCenterAnchor we split like in useSplitDropZone, otherwise just 50/50
        if (options.hasCenterAnchor) {
          const edgeZone = options.orientation == Orientation.HORIZONTAL ? targetRect.width : targetRect.height;
          const centerZone = edgeZone * SPLIT_EDGE_ZONE_FRACTION;
          const anchor =
            cursorP <= targetStart + centerZone ? "start" : cursorP >= targetEnd - centerZone ? "end" : "center";
          return { targetId, anchor };
        } else {
          const anchor = cursorP <= (targetStart + targetEnd) / 2 ? "start" : "end";
          return { targetId, anchor };
        }
      }
    }

    const singleDropZone = getSingleActiveDropZone();
    if (options?.defaultToEdge) {
      // otherwise if we have targets we attribute to first/last target (sorted by position)
      const targetsSorted = Object.entries(options.targets.value).sort(([targetId, targetEl]) => {
        const targetRect = getElement(targetEl)!.getBoundingClientRect();
        return options.orientation == Orientation.HORIZONTAL ? targetRect.left : targetRect.top;
      });
      if (targetsSorted.length > 0) {
        // anchor=end assumes that targets are positioned start to end in the container
        if (singleDropZone.anchor == "start") {
          return { targetId: targetsSorted[0][0], anchor: "end" };
        } else {
          return { targetId: targetsSorted[targetsSorted.length - 1][0], anchor: "end" };
        }
      }
    }

    // else we attribute to entire container
    return { targetId: null, anchor: singleDropZone.anchor };
  }

  const position = useMouse();
  const activeDropZone: Ref<{ anchor: MultiAnchor; targetId: string | null } | null> = computed(() => {
    if (singleDropZone.value == null) return null;
    const activeDropZone = getActiveDropZone();
    if (
      activeDropZone.targetId != null &&
      options.allowDrop?.(activeDragged.value!, activeDropZone.anchor, activeDropZone.targetId) === false
    )
      return null;
    return activeDropZone;
  });

  return { activeDropZone };
}
