<script lang="ts" setup>
import { makeNodeName } from "@/language/core/node";
import { newChangeId } from "@/language/runtime/transaction";
import { ActionType, ImplementationData, NodeReferenceData, NodeType, ObjectType, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { isDragging, startDraggingIfAllowed, useMultiDropZone } from "@/ui/drag";
import { ACTION_SIZE } from "@/ui/flow";
import { generateOrderKey } from "@/utils/fractional";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Action from "@/views/nodes/Action.vue";
import { computed, Ref, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = toRef(props, "nodePtr");
const preparedConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const implementation = graph.getRef(nodePtr) as Ref<ImplementationData | null>;
const actions = graph.getChildrenRef(nodePtr, NodeType.ACTION);

const headerRef = ref<InstanceType<typeof InlineHeader> | null>(null);
const containerRef = ref<HTMLElement | null>(null);
const actionRefs: Ref<Record<string, InstanceType<typeof Action> | null>> = ref({});

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

// drag and drop
const { activeDropZone } = useMultiDropZone({
  name: "action",
  container: containerRef,
  targetsInOrder: computed(() => actions.value.map((action) => action.id)),
  targetsById: actionRefs,
  kinds: ["node", "selection"],
  metatypes: [NodeType.ACTION],
  fallbackToClosest: true,
  allowDrop: (dragged, anchor, targetId) => {
    return true;
  },
  onDrop: (dragged, anchor, targetId, event) => {},
});

/** Creates a new Action. */
function createAction() {
  if (implementation.value == null) throw new Error("no implementation");
  const orderKey = generateOrderKey(actions.value[actions.value.length - 1]?.orderKey ?? null, null);

  // create
  let tx = connection.tx;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const action = tx.create({
    metatype: NodeType.ACTION,
    name: makeNodeName(graph, {
      metatype: ObjectType.ACTION,
      type: ActionType.CODE,
      parentPtr: nodePtr.value,
    }),
    parentPtr: nodePtr.value,
    benchPtr: implementation.value.benchPtr,
    packagePtr: implementation.value.packagePtr,
    type: ActionType.CODE,
    orderKey,
  });
  canvas.inspect({ node: action });
  return action;
}

defineExpose<ViewExpose>({ self, id, focus });
</script>

<template>
  <div>
    <InlineHeader
      v-if="nodePtr"
      ref="headerRef"
      :self="self"
      :node="implementation"
      :connection="preparedConnection"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      :is-root="isRoot"
      :is-inline="isInline"
      :is-minimal="isMinimal"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <!-- Action grid -->
    <div
      ref="containerRef"
      class="relative gap-x-2 gap-y-2 py-2"
      :style="{
        display: 'grid',
        gridTemplateColumns: `repeat(auto-fill, minmax(${ACTION_SIZE.width}px, 1fr))`,
      }"
    >
      <!-- Actions -->
      <div v-for="action in actions" :key="action.id" class="relative">
        <!-- Drop indicator -->
        <div
          v-if="activeDropZone?.targetId === action.id"
          class="absolute z-10 rounded bg-gray-400"
          :class="[activeDropZone.anchor === 'start' ? '-left-[6px]' : '-right-[6px]', 'top-0 h-full w-1']"
        />
        <Action
          :id="action.id"
          :ref="(el: any) => (el ? (actionRefs[action.id] = el) : delete actionRefs[action.id])"
          :node-ptr="toNodeRef(action)"
          class="w-full transition-opacity duration-150"
          :class="{ 'opacity-50': isDragging(action) }"
          :style="{ height: ACTION_SIZE.height + 'px' }"
          data-suppress-drag="select"
          :draggable="true"
          @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, action)"
        />
      </div>
      <!-- Add action -->
      <button
        class="group/action flex w-full flex-row items-center gap-x-2.5 rounded border border-dashed border-gray-200 px-1 py-1 text-left transition-colors duration-150 hover:border-gray-400 hover:bg-gray-100"
        :style="{ height: ACTION_SIZE.height + 'px' }"
        @click.stop.prevent="createAction"
      >
        <div class="flex h-10 w-10 items-center justify-center rounded bg-gray-100">
          <i
            class="fas fa-plus text-lg text-gray-400 transition-colors duration-150 group-hover/action:text-gray-700"
          />
        </div>
        <div class="flex flex-1 flex-col">
          <span class="text-gray-400 transition-colors duration-150 group-hover/action:text-gray-700">Action</span>
          <span class="text-gray-400">...</span>
        </div>
      </button>
    </div>
  </div>
</template>
