import {
  FileFormat,
  FileType,
  NodeType,
  Orientation,
  SelectionData,
  StructType,
  type AnyNodeData,
  type NodeReferenceData,
} from "@/proto/wire";
import { isNodeRef, isStruct, toNodeRef, type AnyNodeReferenceData } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import { getElement, getElementRef } from "@/utils/element";
import { log } from "@/utils/log";
import { uuidt } from "@/utils/uuidt";
import { tryOnBeforeUnmount, useEventListener, useMouse, useMouseInElement, type MaybeElement } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, shallowRef, toRef, unref, watch, type MaybeRef, type Ref } from "vue";

// NOTE :Architecture: we can probably generalise drag & selection targets into a single system?
//  (also canvas.registerView, View.nodePtr and data-node-id annotations seem very relevant)
// NOTE: for now the drag/drop system is only expected to work within a single bench-web instance
//  (i.e. not across instances, but it should work across with other applications)

const DRAGGED_KINDS = ["node", "selection", "file"] as const;
export type DraggedKind = (typeof DRAGGED_KINDS)[number];
export type DraggedContent =
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
      files: FileList | undefined; // only on drop
    };
export type Dragged = {
  id: string;
  trigger: HTMLElement | undefined;
} & DraggedContent;

// NOTE: we render the drag image globally in DragOverlay.vue
const dragImageRef = ref<HTMLElement | null>(null);

export function _setDragImage(image: HTMLElement | null) {
  dragImageRef.value = image;
}

export const activeDragged: Ref<Dragged | null> = shallowRef(null);
const lastDraggedAt: Ref<DateTime | null> = shallowRef(null);
const dropZones: Ref<Record<number, DropZone>> = shallowRef({});
const dropZonesByElement: Map<HTMLElement | SVGElement, DropZone> = new Map();
const activeDropZone: Ref<DropZone | null> = ref(null);

/** Start dragging the given thing. Sets 'activeDragged' (can only drag one thing at a time). */
export function startDragging(
  event: DragEvent,
  graph: ReadNodeGraph,
  data: AnyNodeData | AnyNodeReferenceData | SelectionData | DraggedContent,
) {
  const trigger = event.target as HTMLElement;
  let dragged: Dragged;
  if ("metatype" in data) {
    if (isNodeRef(data)) {
      dragged = {
        id: uuidt(),
        trigger,
        kind: "node",
        node: data,
        nodes: [graph.getOrError(data)],
      };
    } else if (isStruct(data, StructType.SELECTION)) {
      dragged = {
        id: uuidt(),
        trigger,
        kind: "selection",
        selection: data,
        nodes: data.nodesPtr.map((n) => graph.getOrError(n)),
      };
    } else {
      dragged = {
        id: uuidt(),
        trigger,
        kind: "node",
        node: toNodeRef(data),
        nodes: [data],
      };
    }
  } /* DraggedContent */ else {
    dragged = { id: uuidt(), trigger, ...data };
  }

  const dt = event.dataTransfer;
  if (!dt) throw new Error(`no dataTransfer on event: ${event}`);
  dt.setData("application/symbolx.bench." + dragged.id, JSON.stringify({ ...dragged, trigger: null, nodes: [] }));
  dt.setDragImage(dragImageRef.value!, -10, 0);

  trigger.dataset.dragging = "true";
  if (activeDragged.value != null) log.warn("drag.alreadyExists", activeDragged);
  activeDragged.value = dragged;
  lastDraggedAt.value = DateTime.now();
  log.trace("drag.start", dragged);
}

/** Stops dragging the current thing. */
function stopDragging() {
  if (activeDragged.value?.trigger != null) {
    delete activeDragged.value.trigger?.dataset.dragging;
  }
  activeDragged.value = null;
  lastDraggedAt.value = null;
  activeDropZone.value = null;
}

/** Gets the current dragged thing. Must match 'activeDragged'. */
function getDragged(event: DragEvent): DraggedContent | null {
  // get dragged metatype while dragging (can only read keys set in startDragging above)
  if (event.dataTransfer?.types == null) return null;

  // check if it's one of our dragged items
  const draggedId = event.dataTransfer.types
    .find((t) => t.startsWith("application/symbolx.bench."))
    ?.split(".")
    .pop();
  if (draggedId != null) {
    if (activeDragged.value?.id == draggedId) {
      return activeDragged.value;
    } else {
      log.warn("drag.notFound", draggedId, activeDragged);
    }
  }

  // might be a file drop
  if (event.dataTransfer.types.includes("Files")) {
    return { kind: "file", files: event.dataTransfer.files };
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
  fileTypes?: MaybeRef<FileType[]>;
  fileFormats?: MaybeRef<FileFormat[]>;
  /** Whether the drop zone is enabled. */
  isEnabled?: Ref<boolean>;
};

//
// Drop zones
// NOTE: to handle hierarchical drop zones, we register them globally here and query against thee dropZones registry.
//

type DropZone = DropOptions & {
  id: number;
  containerEl: Ref<HTMLElement | SVGElement | null>;
  onDrop?: (dragged: DraggedContent, event: DragEvent) => void;
};
let dropZoneId = 0;
function newDropZoneId(): number {
  return dropZoneId++;
}

/** Whether dropping the dragged thing into this zone is possible */
function isDropCompatible(zone: DropZone, dragged: DraggedContent): boolean {
  if (dragged == null) return false;
  const kinds = unref(zone.kinds);
  if (kinds != null && !kinds.includes(dragged.kind)) return false;
  const metatypes = unref(zone.metatypes);
  if (metatypes != null && dragged.kind == "node" && !metatypes.includes(dragged.node.type)) return false;
  return true;
}

/** Traverses the event targets up to find a drop zone that can accept the dragged thing (if any). */
function findCompatibleDropZone(el: HTMLElement | SVGElement | null, dragged: DraggedContent): DropZone | null {
  while (el) {
    const zone = dropZonesByElement.get(el);
    if (zone && isDropCompatible(zone, dragged)) return zone;
    el = el.parentElement;
  }
  return null;
}

/** Updates the active dragging on any drag event. */
function updateDragging(event: DragEvent) {
  const dragged = getDragged(event);
  if (dragged == null) {
    if (activeDropZone.value != null) stopDragging();
    return;
  }
  event.preventDefault();
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone?.id !== activeDropZone.value?.id) {
    activeDropZone.value = zone;
    log.trace("drag.activeZone", zone);
  }
  lastDraggedAt.value = DateTime.now();
  // schedule check if still active
  //  (this handles the case where something external is dragged into bench-web and we don't get a dragleave/drop/dragend)
  setTimeout(() => {
    if (activeDropZone.value == null) return;
    if (DateTime.now().diff(lastDraggedAt.value!).milliseconds > 200) {
      stopDragging();
    }
  }, 250);
}

useEventListener("dragenter", updateDragging);
useEventListener("dragover", updateDragging);
useEventListener("dragleave", updateDragging);
useEventListener("drop", (event) => {
  const dragged = getDragged(event);
  if (dragged == null) return;
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone) {
    log.debug("drag.drop", dragged, zone);
    zone.onDrop?.(dragged, event);
  }
  stopDragging();
});
useEventListener("dragend", stopDragging);

/**
 * Track certain drop events in a target region.
 */
export function useDropZone(
  options: DropOptions & {
    onDrop?: (dragged: DraggedContent, event: DragEvent) => void;
  },
): { isInDropZone: Ref<boolean> } {
  const isEnabled = options.isEnabled ?? ref(true);

  // create & register/deregister zone
  const zone: DropZone = {
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
    isInDropZone: computed(() => isEnabled.value && activeDropZone.value?.id == zone.id),
  };
}

/**
 * Track certain drop zone events in a single target region.
 */
export function useSingleDropZone(
  options: DropOptions & {
    orientation: MaybeRef<Orientation>;
    onDrop?: (dragged: DraggedContent, anchor: "start" | "end", event: DragEvent) => void;
  },
): { activeDropZone: Ref<{ anchor: "start" | "end" } | null>; getActiveDropZone: () => { anchor: "start" | "end" } } {
  const { isInDropZone } = useDropZone({
    ...options,
    onDrop: (dragged, event) => {
      options.onDrop?.(dragged, getActiveDropZone().anchor, event);
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

export type MultiAnchor = "start" | "center" | "end";
/**
 * Track certain drop zone events across dynamic target regions in a single parent container.
 */
export function useMultiDropZone(
  options: DropOptions & {
    targets: Ref<Record<string, MaybeElement>>;
    orientation: MaybeRef<Orientation>;
    fallbackToClosest?: boolean;
    hasCenterAnchor?: boolean;
    allowDrop?: (dragged: DraggedContent, anchor: MultiAnchor, targetId: string, event?: DragEvent) => boolean;
    onDrop?: (dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) => void;
  },
): { activeDropZone: Ref<{ anchor: MultiAnchor; targetId: string | null } | null> } {
  const { activeDropZone: singleDropZone, getActiveDropZone: getSingleActiveDropZone } = useSingleDropZone({
    ...options,
    orientation: options.orientation,
    onDrop: (dragged, _, event) => {
      const { anchor, targetId } = getActiveDropZone()!;
      if (targetId == null || options.allowDrop?.(dragged, anchor, targetId, event) !== false) {
        options.onDrop?.(dragged, anchor, targetId, event);
      }
    },
  });

  function getActiveDropZone(): { anchor: MultiAnchor; targetId: string | null } {
    // find directly hit zone (and closest as fallback)
    const cursor = { x: position.x.value, y: position.y.value };
    let closest: { anchor: MultiAnchor; targetId: string; distance: number } | null = null;
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
      } else {
        // check if it's new closest
        const distance = Math.min(Math.abs(cursorP - targetStart), Math.abs(cursorP - targetEnd));
        if (closest == null || distance < closest.distance) {
          const anchor = cursorP < targetStart ? "start" : "end";
          closest = { anchor, targetId, distance };
        }
      }
    }

    // fallback to closest if possible
    if (options?.fallbackToClosest && closest != null) return closest;
    // else we attribute to entire container
    return { targetId: null, anchor: getSingleActiveDropZone().anchor };
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

export type SplitAnchor = "center" | "left" | "top" | "right" | "bottom";
export const SPLIT_ANCHOR_OPPOSITE: Record<SplitAnchor, SplitAnchor> = {
  center: "center",
  left: "right",
  top: "bottom",
  right: "left",
  bottom: "top",
};
export const SPLIT_EDGE_ZONE_FRACTION = 0.12;

/*
 * Track 'split' container drop events.
 * The left/top/right/bottom fraction percent are the respective zones, the rest is the center zone.
 * If the cursor is in two zones at once, the edge we're closest to wins.
 */
export function useSplitDropZone(
  options: DropOptions & {
    onDrop?: (dragged: DraggedContent, anchor: SplitAnchor, event: DragEvent) => void;
  },
): {
  activeDropZone: Ref<{ anchor: SplitAnchor; splitClass: string } | null>;
} {
  const { isInDropZone } = useDropZone({
    ...options,
    onDrop: (dragged, event) => {
      options.onDrop?.(dragged, getActiveDropZone().anchor, event);
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
