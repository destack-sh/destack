<script lang="ts" setup>
import { makeNodeName, packSubnode } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { newChangeId } from "@/language/runtime/transaction";
import {
  ActionData,
  ActionType,
  BenchType,
  KitData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PickerVariant,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ACTION_SIZE } from "@/ui/flow";
import { generateOrderKey } from "@/utils/fractional";
import Grid from "@/views/builtins/Grid.vue";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Action from "@/views/nodes/Action.vue";
import { useElementSize } from "@vueuse/core";
import { ComponentPublicInstance, Ref, ref, toRef } from "vue";
const GUTTER_WIDTH = 60;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedNodeConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// state
const nodePtr = toRef(props, "nodePtr");
const preparedConnection = props.preparedConnection ?? useAutoConnection(nodePtr);
const { graph, connection } = preparedConnection;
const implementation = graph.getRef(nodePtr) as Ref<KitData | null>;
const actions = graph.getChildrenRef(nodePtr, NodeType.ACTION);

// view
const containerRef = ref<HTMLElement | null>(null);
const containerSize = useElementSize(containerRef);
const headerRef = ref<InstanceType<typeof InlineHeader> | null>(null);
const gridRef = ref<ComponentPublicInstance<typeof Grid> | null>(null);

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

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
  <div ref="containerRef" class="">
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
      :width="containerSize.width.value - GUTTER_WIDTH * 2"
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
    <Grid
      :id="`${id}-grid`"
      ref="gridRef"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      :element-type="NodeType.ACTION"
      :element-size="ACTION_SIZE"
      :is-root="isRoot"
      :style="{
        marginLeft: isRoot ? `${GUTTER_WIDTH}px` : undefined,
        marginRight: isRoot ? `${GUTTER_WIDTH}px` : undefined,
      }"
      @create="createActionPopover"
    >
      <template #element="{ element, elementRef, nodePtr, isDragging, elementSize, startDragging }">
        <Action
          :id="element.id"
          :ref="elementRef"
          :node-ptr="nodePtr"
          :prepared-connection="preparedConnection"
          class="w-full transition-opacity duration-150"
          :class="{ 'opacity-50': isDragging }"
          :style="{ height: elementSize.height + 'px' }"
          data-suppress-drag="select"
          :draggable="true"
          @dragstart.stop="startDragging"
        />
      </template>
    </Grid>
  </div>
</template>
