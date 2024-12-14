import {
  FileType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  SelectionData,
  StructType,
  type AnyNodeData,
} from "@/proto/wire";
import { contentEquals, isNode, isNodeRef, isStruct, toNodeRef } from "@/proto/wiring";
import { canvas, supergraph } from "@/system/globals";
import { makeSelection } from "@/ui/view";
import { getElement, getElementRef } from "@/utils/element";
import { groupByList } from "@/utils/functools";
import { log } from "@/utils/log";
import { uuidt } from "@/utils/uuidt";
import SelectionZone from "@/views/builtins/SelectionOverlay.vue";
import { ViewComponent } from "@/views/common";
import { tryOnBeforeUnmount, useEventListener, useMouse, useMouseInElement, type MaybeElement } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, shallowRef, toRef, triggerRef, unref, watch, type MaybeRef, type Ref } from "vue";

//
// Drag
//

const DRAG_KINDS = ["node", "selection", "file"] as const;
export type DragKind = (typeof DRAG_KINDS)[number];
export type DragContent =
  | { kind: "node"; node: NodeReferenceData; nodes: AnyNodeData[] }
  | { kind: "selection"; selection: SelectionData; nodes: AnyNodeData[] }
  | {
      kind: "file"; // native browser file
      files: FileList | undefined; // only on drop
    };
export type Drag = {
  trigger?: HTMLElement | undefined;
} & DragContent;

// NOTE: we render the drag image globally in DragOverlay.vue
const dragImageRef = ref<HTMLElement | null>(null);
export function _setDragImage(image: HTMLElement | null) {
  dragImageRef.value = image;
}

export const activeDrag: Ref<Drag | null> = shallowRef(null);
const lastDraggedAt: Ref<DateTime | null> = shallowRef(null);

export const DRAG_DISALLOWED_ELEMENTS = new Set(["input", "textarea", "contenteditable"]);

/** Checks whether the given node is currently being dragged */
export function isDragging(node: AnyNodeData | NodeReferenceData) {
  return (
    (activeDrag.value?.kind == "node" && activeDrag.value.node.id == node.id) ||
    (activeDrag.value?.kind == "selection" && activeDrag.value.nodes.some((n) => n.id == node.id))
  );
}

/** Checks whether the given element may be dragged. */
export function isDragAllowed(element: HTMLElement | SVGElement | null, what: "drag" | "select"): boolean {
  if (element == null || DRAG_DISALLOWED_ELEMENTS.has(element.tagName.toLowerCase())) return false;
  // check for 'data-suppress-drag' attribute in containing elements
  let el: HTMLElement | SVGElement | null = element;
  while (el != null) {
    const suppress = el.getAttribute("data-suppress-drag");
    if (suppress == "both" || suppress == what) {
      return false;
    }
    el = el.parentElement as HTMLElement | null;
  }
  return true;
}

/** Start dragging the given thing if it's not a disallowed element (like an input). */
export function startDraggingIfAllowed(event: DragEvent, data: AnyNodeData | NodeReferenceData): boolean {
  const trigger = event.target as HTMLElement;
  if (!isDragAllowed(trigger, "drag")) {
    log.trace("drag.start.disallowed", trigger);
    event.preventDefault();
    return false;
  } else {
    return startDragging(event, data);
  }
}

/**
 * Start dragging the given thing.
 * If we have an active selection and the thing is part of the selection, drag the selection.
 * Sets 'activeDragged' (can only drag one thing at a time). */
export function startDragging(event: DragEvent, data: AnyNodeData | NodeReferenceData | SelectionData): boolean {
  const trigger = event.target as HTMLElement;
  let dragged: Drag;
  if (isNodeRef(data)) {
    data = supergraph.getOrError(data);
  }
  if (isNode(data)) {
    if (canvas.isSelected(data)) {
      // promote to selection
      const nodes = supergraph.getManyMaybe(canvas.selection?.nodesPtr ?? []);
      dragged = { trigger, kind: "selection", selection: canvas.selection!, nodes };
    } else {
      // just drag the node
      canvas.select([data]);
      dragged = { trigger, kind: "node", node: toNodeRef(data), nodes: [data] };
    }
  } else if (isStruct(data, StructType.SELECTION)) {
    // already a selection
    dragged = { trigger, kind: "selection", selection: data, nodes: supergraph.getManyMaybe(data.nodesPtr) };
  } else {
    throw new Error(`unsupported drag data: ${data}`);
  }

  const dt = event.dataTransfer;
  if (!dt) throw new Error(`no dataTransfer on event: ${event}`);
  dt.setData("application/symbol.bench." + uuidt(), JSON.stringify({ ...dragged, trigger: null, nodes: [] }));
  dt.setDragImage(dragImageRef.value!, -10, 0);

  trigger.dataset.dragging = "true";
  if (activeDrag.value != null) log.warn("drag.alreadyExists", activeDrag);
  activeDrag.value = dragged;
  lastDraggedAt.value = DateTime.now();
  log.trace("drag.start", dragged);
  return true;
}

/** Stops dragging the current thing. */
function stopDragging() {
  if (activeDrag.value?.trigger != null) {
    delete activeDrag.value.trigger?.dataset.dragging;
  }
  activeDrag.value = null;
  lastDraggedAt.value = null;
  activeDropZone.value = null;
}

/** Gets the current dragged thing. Must match 'activeDragged'. */
function getDragContent(event: DragEvent): DragContent | null {
  // get dragged metatype while dragging (can only read keys set in startDragging above)
  if (event.dataTransfer?.types == null) return null;

  // might be a new file drop
  if (event.dataTransfer.types.includes("Files")) {
    return { kind: "file", files: event.dataTransfer.files };
  }

  // check if it's one of our current dragged items
  if (activeDrag.value != null) {
    return activeDrag.value;
  }

  // something else
  return null;
}

//
// Drop
// NOTE: to handle hierarchical drop zones, we register them globally here and query against thee dropZones registry.
//

const dropZones: Ref<Record<number, DropZone>> = shallowRef({});
const dropZonesByContainerEl: Map<HTMLElement | SVGElement, DropZone> = new Map();
const activeDropZone: Ref<DropZone | null> = ref(null);

/** General options for any drop zone. */
type DropOptions = {
  /** Name of the drop zone for debugging */
  name: string;
  /** The top level container for the zone. */
  container: Ref<MaybeElement>;
  /** The kinds of supported drag kinds. */
  kinds?: MaybeRef<DragKind[]>;
  /** The metatypes of supported drag nodes (for Dragged with nodes). */
  metatypes?: MaybeRef<NodeType[]>;
  /** The allowed file types for file drops. */
  fileTypes?: MaybeRef<FileType[]>;
  /** Whether the drop zone is enabled. */
  isEnabled?: Ref<boolean>;
};
type DropZone = DropOptions & {
  id: number;
  containerEl: Ref<HTMLElement | SVGElement | null>;
  onDrop?: (dragged: DragContent, event: DragEvent) => void;
};
let dropZoneId = 0;
function newDropZoneId(): number {
  return dropZoneId++;
}

/** Whether dropping the dragged thing into this zone is possible */
function isDropCompatible(zone: DropZone, dragged: DragContent): boolean {
  if (dragged == null) return false;
  const kinds = unref(zone.kinds);
  if (kinds != null && !kinds.includes(dragged.kind)) return false;
  const metatypes = unref(zone.metatypes);
  if (metatypes != null && dragged.kind == "node" && !metatypes.includes(dragged.node.nodeType)) return false;
  return true;
}

/** Traverses the event targets up to find a drop zone that can accept the dragged thing (if any). */
function findCompatibleDropZone(el: HTMLElement | SVGElement | null, dragged: DragContent): DropZone | null {
  while (el) {
    const zone = dropZonesByContainerEl.get(el);
    if (zone && isDropCompatible(zone, dragged)) return zone;
    el = el.parentElement;
  }
  return null;
}

/** Updates the active dragging on any drag event. */
function updateDragging(event: DragEvent) {
  const dragged = getDragContent(event);
  if (dragged == null) {
    if (activeDropZone.value != null) {
      stopDragging();
    }
    return;
  }
  event.preventDefault();

  // update dragged
  if (activeDrag.value == null) {
    activeDrag.value = dragged;
    log.trace("drag.activeDragged", dragged);
  }

  // update zone
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone?.id !== activeDropZone.value?.id) {
    activeDropZone.value = zone;
    log.trace("drag.activeZone", zone);
  }

  // schedule check if still active
  //  (this handles the case where something external is dragged into bench-web and we don't get a dragleave/drop/dragend)
  lastDraggedAt.value = DateTime.now();
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
  const dragged = getDragContent(event) ?? activeDrag.value;
  if (dragged == null) return;
  const zone = findCompatibleDropZone(event.target as HTMLElement | SVGElement, dragged);
  if (zone) {
    log.info("drag.drop", dragged, zone);
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
    onDrop?: (dragged: DragContent, event: DragEvent) => void;
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
      if (oldEl) dropZonesByContainerEl.delete(oldEl);
      if (newEl) dropZonesByContainerEl.set(newEl, zone);
    },
    { immediate: true },
  );
  tryOnBeforeUnmount(() => {
    delete dropZones.value[zone.id];
    if (zone.containerEl.value) dropZonesByContainerEl.delete(zone.containerEl.value);
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
    onDrop?: (dragged: DragContent, anchor: "start" | "end", event: DragEvent) => void;
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
    allowDrop?: (dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent) => boolean;
    onDrop?: (dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) => void;
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
        // if hasCenterAnchor we consider edges only like in useSplitDropZone, otherwise just 50/50
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
    if (options.allowDrop?.(activeDrag.value!, activeDropZone.anchor, activeDropZone.targetId) === false) return null;
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
    onDrop?: (dragged: DragContent, anchor: SplitAnchor, event: DragEvent) => void;
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

//
// Selection (drag to select)
//

type SelectionZoneOptions = {
  isEnabled?: Ref<boolean>;
};

export type SelectionZone = {
  id: number;
  containerEl: Ref<HTMLElement | SVGElement | null>;
  overlayEl: Ref<InstanceType<typeof SelectionZone> | null>;
  options: SelectionZoneOptions;
};

type SelectionTarget = {
  clientX: number;
  clientY: number;
  element: HTMLElement | SVGElement;
};

let selectionZoneId = 0;
function newSelectionZoneId() {
  return selectionZoneId++;
}
let selectionId = 0;
function newSelectionId() {
  return selectionId++;
}

const selectionZones: Ref<Record<number, SelectionZone>> = shallowRef({});
const selectionZonesByContainerEl = new Map<HTMLElement | SVGElement, SelectionZone>();
export const activeSelection: Ref<{
  id: number;
  sourceZone: SelectionZone;
  start: SelectionTarget;
  end: SelectionTarget;
  activeZone: SelectionZone | null;
  activeOverlay: { x: number; y: number; width: number; height: number } | null;
} | null> = shallowRef(null);

export function isSelecting(zone?: SelectionZone) {
  if (zone == null) {
    return activeSelection.value != null;
  } else {
    return activeSelection.value?.activeZone?.id == zone.id;
  }
}

/** Creates a selection zone. */
export function useSelectionZone(
  options: SelectionZoneOptions & {
    containerEl: Ref<HTMLElement | SVGElement | null>;
    overlayEl: Ref<InstanceType<typeof SelectionZone> | null>;
  },
): SelectionZone {
  const isEnabled = options?.isEnabled ?? ref(true);
  const zone: SelectionZone = {
    id: newSelectionZoneId(),
    containerEl: options.containerEl,
    overlayEl: options.overlayEl,
    options: { ...options, isEnabled },
  };

  // register/deregister
  selectionZones.value[zone.id] = zone;
  triggerRef(selectionZones);
  tryOnBeforeUnmount(() => {
    delete selectionZones.value[zone.id];
    triggerRef(selectionZones);
  });
  watch(zone.containerEl, (newEl, oldEl) => {
    if (oldEl) selectionZonesByContainerEl.delete(oldEl);
    if (newEl) selectionZonesByContainerEl.set(newEl, zone);
  });

  return zone;
}

function eventToTarget(event: MouseEvent): SelectionTarget {
  return { clientX: event.clientX, clientY: event.clientY, element: event.target as HTMLElement | SVGElement };
}

/** Starts selecting at the given position. */
export function startSelecting(zone: SelectionZone, event: MouseEvent) {
  if (activeSelection.value != null) return;
  if (canvas.selection != null) {
    clearSelection();
  }
  activeSelection.value = {
    id: newSelectionId(),
    sourceZone: zone,
    start: eventToTarget(event),
    end: eventToTarget(event),
    activeZone: zone,
    activeOverlay: null,
  };
  updateSelecting(event);
  log.trace("select.start", activeSelection.value);
}

/** Starts selecting if allowed at the given position. */
export function startSelectingIfAllowed(zone: SelectionZone, event: MouseEvent) {
  // ignore contextmenu/right-click
  if (event.button == 2) return;
  if (isDragAllowed(event.target as HTMLElement | SVGElement | null, "select")) {
    startSelecting(zone, event);
  }
}

/** Gets all elements intersecting the given rectangle. */
function getIntersectingNodes(
  containerEl: HTMLElement,
  x: number,
  y: number,
  width: number,
  height: number,
): NodeReferenceData[] {
  const x2 = x + width;
  const y2 = y + height;
  const allElements = containerEl.querySelectorAll<HTMLElement>("*");
  const nodesById: Record<string, NodeReferenceData> = {};

  for (let i = 0; i < allElements.length; i++) {
    const el = allElements[i];
    const elBounding = el.getBoundingClientRect();

    // check for intersecting nodes
    if (
      elBounding.left <= x2 &&
      elBounding.right >= x &&
      elBounding.top <= y2 &&
      elBounding.bottom >= y &&
      el.dataset?.["ignoreElement"] != "self"
    ) {
      let nodePtr: NodeReferenceData | null = null;
      if (el.dataset?.["nodeId"] != null) {
        nodePtr = {
          metatype: ObjectType.NODE_REFERENCE,
          nodeType: Number(el.dataset["nodeType"]),
          id: el.dataset["nodeId"],
          ck: el.dataset["nodeCk"],
        };
      } else if ((el as any).__viewComponent != null) {
        const component = (el as any).__viewComponent as ViewComponent;
        if (component.props.nodePtr != null) {
          nodePtr = component.props.nodePtr;
        }
      }
      if (nodePtr != null) {
        // resolve against supergraph (to get full node ref)
        const node = supergraph.get(nodePtr);
        if (node != null) nodePtr = toNodeRef(node);
        nodesById[nodePtr.id!] = nodePtr;
      }
    }
  }

  return Object.values(nodesById);
}

// rank for aggregating selections into higher rank nodes (sometimes)
// lower ranks are more specific, higher ranks are broader
const SELECTION_RANK_DEFAULT = 1;
const SELECTION_RANK_BY_NODE_TYPE: Partial<Record<NodeType, number>> = {
  [NodeType.FIELD]: 2,
  [NodeType.VIEW]: 3,
  [NodeType.STEP]: 5,
  [NodeType.PIPE]: 5,
  [NodeType.RECORD]: 5,
  [NodeType.BLOCK]: 10,
};

const SELECTION_MIN_SIZE = 5;

/** Updates selecting to the given position. */
export function updateSelecting(event: MouseEvent) {
  if (activeSelection.value == null) return;

  activeSelection.value.end = eventToTarget(event);
  const { sourceZone, start, end } = activeSelection.value;

  // if start and end are too close, consider the selection empty
  if (
    Math.abs(start.clientX - end.clientX) < SELECTION_MIN_SIZE &&
    Math.abs(start.clientY - end.clientY) < SELECTION_MIN_SIZE
  ) {
    activeSelection.value.activeZone = null;
    activeSelection.value.activeOverlay = null;
    triggerRef(activeSelection);
    return;
  }

  // figure out active zone (common container of start and end element)
  let activeZone: SelectionZone | null = null;
  for (const zone of Object.values(selectionZones.value)) {
    if (
      zone.containerEl.value != null &&
      zone.containerEl.value.contains(start.element) &&
      zone.containerEl.value.contains(end.element)
    ) {
      activeZone = zone;
      break;
    }
  }
  activeSelection.value.activeZone = activeZone;

  // position overlay in active zone
  if (activeZone != null && activeZone.containerEl.value != null) {
    const containerRect = activeZone.containerEl.value!.getBoundingClientRect();
    const overlay = {
      x: Math.min(start.clientX, end.clientX) - containerRect.left,
      y: Math.min(start.clientY, end.clientY) - containerRect.top,
      width: Math.abs(start.clientX - end.clientX),
      height: Math.abs(start.clientY - end.clientY),
    };
    activeSelection.value.activeOverlay = overlay;
  }
  triggerRef(activeSelection);

  // figure out new selection
  let selection: SelectionData | undefined = undefined;
  if (activeZone?.containerEl.value != null) {
    const overlay = {
      x: Math.min(start.clientX, end.clientX),
      y: Math.min(start.clientY, end.clientY),
      width: Math.abs(start.clientX - end.clientX),
      height: Math.abs(start.clientY - end.clientY),
    };
    const intersectingNodes = getIntersectingNodes(
      activeZone.containerEl.value as HTMLElement,
      overlay.x,
      overlay.y,
      overlay.width,
      overlay.height,
    );
    let selectionNodes: NodeReferenceData[];
    if (intersectingNodes.some((n) => SELECTION_RANK_BY_NODE_TYPE[n.nodeType] != null)) {
      // pick the highest rank where there is more than one node
      //  otherwise pick the most specific (lowest) rank
      const nodesByRank = groupByList(
        intersectingNodes,
        (n) => SELECTION_RANK_BY_NODE_TYPE[n.nodeType] ?? SELECTION_RANK_DEFAULT,
      );
      const maxRankWithMultipleNodes = Object.keys(nodesByRank)
        .map(Number)
        .sort((a, b) => b - a)
        .find((r) => nodesByRank[r].length > 1);
      if (maxRankWithMultipleNodes != null) {
        // pick the highest rank where there is more than one node
        selectionNodes = nodesByRank[maxRankWithMultipleNodes];
      } else {
        // pick the most specific (lowest) rank
        selectionNodes = nodesByRank[Math.min(...Object.keys(nodesByRank).map(Number))];
      }
    } else {
      selectionNodes = intersectingNodes;
    }
    selection = makeSelection(selectionNodes);
  }

  // update selection (if any)
  if (
    activeZone != null &&
    ((selection != null && canvas.selection == null) ||
      (selection == null && canvas.selection != null) ||
      (selection != null && canvas.selection != null && !contentEquals(canvas.selection, selection)))
  ) {
    // update selection in active zone
    canvas.select(selection, { debounce: "long" });
  }
}

/** Stops selecting. */
export function stopSelecting() {
  activeSelection.value = null;
  log.trace("select.stop");
}

export function clearSelection() {
  if (canvas.selection != null) {
    canvas.select(undefined, { debounce: "long" });
    activeSelection.value = null;
  }
  log.trace("select.clear");
}

useEventListener("mousemove", updateSelecting);
useEventListener("mouseup", stopSelecting);
