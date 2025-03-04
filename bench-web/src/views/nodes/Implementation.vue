<script lang="ts" setup>
import { cloneNode, makeNodeName, moveNode, packSubnode } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { newChangeId } from "@/language/runtime/transaction";
import {
  ActionData,
  ActionType,
  BenchType,
  ImplementationData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PickerVariant,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
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

/** Creates a new Action. */
function createAction(actionIn: Partial<ActionData>) {
  if (implementation.value == null) throw new Error("no implementation");
  const orderKey = generateOrderKey(actions.value[actions.value.length - 1]?.orderKey ?? null, null);

  // create
  let tx = connection.tx;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const type: ActionType = actionIn.type ?? ActionType.CODE;
  const action = tx.create({
    metatype: NodeType.ACTION,
    name: makeNodeName(graph, { metatype: ObjectType.ACTION, type, parentPtr: nodePtr.value }),
    type: type as any,
    ...actionIn,
    parentPtr: nodePtr.value,
    benchPtr: implementation.value.benchPtr,
    packagePtr: implementation.value.packagePtr,
    orderKey,
  });
  canvas.inspect({ node: action });
  return action;
}

function createActionPopover(e: MouseEvent) {
  canvas.pushPopover({
    kind: "view",
    trigger: e.target as HTMLElement,
    reference: e.target as HTMLElement,
    component: ViewType.PICKER,
    title: "Add Action",
    placement: "bottom-left",
    offset: "referenceWidth",
    props: {
      valueType: makeType({ kind: TypeKind.ENUM, benchType: BenchType.ACTION_TYPE }),
      subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, {
        variant: PickerVariant.DROPDOWN_LARGE,
      }),
    },
    onApply: (value) => {
      createAction({ type: value });
    },
  });
}

defineExpose<ViewExpose>({ self, id, focus });
</script>

<template>
  <div>
    <!-- Header -->
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
    >
      <template #left="{ style }">
        <!-- For ...? -->
      </template>
      <template #right="{ style }">
        <!-- Create Action -->
        <button
          class="group/button rounded px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
          :class="style == 'block' ? 'opacity-0 group-hover/block:opacity-100 group-hover/header:opacity-100' : ''"
          @click.stop.prevent="createActionPopover"
        >
          <i class="fas fa-plus mr-1.5 text-center" />
          <span class="">Action</span>
        </button>
      </template>
    </InlineHeader>

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
        @click.stop.prevent="(e) => createActionPopover(e)"
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
