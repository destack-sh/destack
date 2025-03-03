<script lang="ts" setup>
import { makeNodeName } from "@/language/core/node";
import { ActionType, ImplementationData, NodeReferenceData, NodeType, ObjectType, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ACTION_SIZE } from "@/ui/flow";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Action from "@/views/nodes/Action.vue";
import { Ref, ref, toRef } from "vue";

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

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

function createAction() {
  if (implementation.value == null) throw new Error("no implementation");
  connection.tx.create({
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
  });
}

defineExpose<ViewExpose>({ self, id });
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
    <div class="flex flex-row flex-wrap gap-x-3 gap-y-2 py-2">
      <!-- Actions -->
      <Action v-for="action in actions" :id="action.id" :key="action.id" :node-ptr="toNodeRef(action)" />
      <!-- Add action -->
      <button
        class="hover:border-gray- flex flex-row items-center justify-center rounded border border-gray-200 transition-colors duration-150 hover:bg-gray-100"
        :style="{
          width: ACTION_SIZE.width + 'px',
          height: ACTION_SIZE.height + 'px',
        }"
        @click="createAction"
      >
        Create Action
      </button>
    </div>
  </div>
</template>
