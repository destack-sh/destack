<script lang="ts" setup generic="T extends NodeType">
import { ViewData, NodeType, NodeTypeMapping } from "@/proto/wire";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, Ref, ref, toRef } from "vue";
import { TypedNodeReferenceData, toNodeRef } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { cloneNode, moveNode } from "@/language/core/node";
import { newChangeId } from "@/language/core/transaction";
import { isDragging, startDraggingIfAllowed, useMultiDropZone } from "@/ui/drag";
import { toCamelName } from "@/language/core/const";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    elementType: T;
    elementSize: { width: number; height: number };
    preparedConnection?: PreparedNodeConnection;
    isRoot?: boolean;
    placeholderText?: string;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits & { (e: "create", event: MouseEvent): void }>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// state
const nodePtr = toRef(props, "nodePtr");
const preparedConnection = props.preparedConnection ?? useAutoConnection(nodePtr);
const { graph, connection } = preparedConnection;
const elements = graph.getChildrenRef(nodePtr, props.elementType);

// view
const gridRef = ref<HTMLElement | null>(null);
const elementRefs: Ref<Record<string, any>> = ref({});

// drag and drop
const { activeDropZone } = useMultiDropZone({
  name: "element",
  container: gridRef,
  targetsInOrder: computed(() => elements.value.map((element) => element.id)),
  targetsById: elementRefs,
  kinds: ["node", "selection"],
  metatypes: [props.elementType],
  fallbackToClosest: true,
  onDrop: (dragged, anchor, targetId, event) => {
    if (targetId == null) return;
    const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
    const targetNode = graph.getOrError({ id: targetId });

    if (dragged.kind == "node") {
      // move node
      let node = graph.getOrError(dragged.node);
      if (event.altKey) {
        // clone node before moving
        node = cloneNode(tx, graph, node, { keepProperties: true });
      }
      moveNode(tx, graph, node, { anchor, target: targetNode });
    } else if (dragged.kind == "selection") {
      // move nodes
      for (let i = 0; i < dragged.nodes.length; i++) {
        let node = graph.getOrError(dragged.nodes[i]);
        if (event.altKey) {
          // clone node before moving
          node = cloneNode(tx, graph, node, { keepProperties: true });
        }
        moveNode(tx, graph, node, {
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? targetNode : graph.getOrError(dragged.nodes[i - 1]),
        });
      }
    }
  },
});

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div
    ref="gridRef"
    class="relative gap-x-2 gap-y-2 py-2"
    :style="{
      display: 'grid',
      gridTemplateColumns: `repeat(auto-fill, minmax(${props.elementSize.width}px, 1fr))`,
    }"
  >
    <!-- Elements -->
    <div v-for="element in elements" :key="element.id" class="relative">
      <!-- Drop indicator -->
      <div
        v-if="activeDropZone?.targetId === element.id"
        class="absolute z-10 rounded-sm bg-gray-400"
        :class="[activeDropZone.anchor === 'start' ? '-left-[6px]' : '-right-[6px]', 'top-0 h-full w-1']"
      />
      <slot
        name="element"
        :element="element"
        :element-ref="(el: any) => (el ? (elementRefs[element.id] = el) : delete elementRefs[element.id])"
        :node-ptr="toNodeRef(element)"
        :is-dragging="isDragging(element)"
        :element-size="props.elementSize"
        :start-dragging="(e: DragEvent) => startDraggingIfAllowed(e, element)"
      ></slot>
    </div>
    <!-- Add element button -->
    <button
      class="group/action flex w-full cursor-pointer flex-row items-center gap-x-2.5 rounded-sm border border-dashed border-gray-200 px-1 py-1 text-left transition-colors duration-150 hover:border-gray-400 hover:bg-gray-100"
      :style="{ height: props.elementSize.height + 'px' }"
      @click.stop.prevent="(e) => emit('create', e)"
    >
      <div class="flex h-10 w-10 items-center justify-center rounded-sm bg-gray-100">
        <i class="fas fa-plus text-lg text-gray-400 transition-colors duration-150 group-hover/action:text-gray-700" />
      </div>
      <div class="flex flex-1 flex-col">
        <span v-if="placeholderText" class="text-sm text-gray-500">{{ placeholderText }}</span>
      </div>
    </button>
  </div>
</template>
