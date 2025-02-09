<script lang="ts" setup>
import { CANVAS_BLOCK_TYPES } from "@/language/core/const";
import { BlockType, NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type FocusAnchor, type NavigationDirection, type ViewEmits, type ViewExposed } from "@/views/common";
import Choice from "@/views/nodes/Choice.vue";
import Database from "@/views/nodes/Database.vue";
import Flow from "@/views/nodes/Flow.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    containerGutterWidth?: number;
  } & Partial<Pick<ViewData, "isMinimal" | "nodePtr">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const blockRef = ref<HTMLElement | null>(null);
const nodeRef: Ref<InstanceType<typeof Choice> | null> = ref(null);

const blockPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedConnection = props.preparedConnection ?? useExistingConnection(blockPtr);
const { graph, connection } = preparedConnection;
const block = graph.getRef(blockPtr, { ignoreAncestors: true });
const nodePtr = computed(() => block.value?.nodePtr);
const node = graph.getRef(nodePtr, { ignoreAncestors: true });
const fields = graph.getChildrenRef(nodePtr, NodeType.FIELD);

const isPage = computed(() => block.value?.type == BlockType.PAGE);
const hasCanvas = computed(() => CANVAS_BLOCK_TYPES.includes(block.value?.type!));
const isInspected = computed(() => canvas.isInspected(blockPtr.value) || canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(blockPtr.value) || canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => state.isSelected(blockPtr.value));

//
// Interaction
//

const actions: Partial<ActionMapImplementation<"space">> & ActionMapImplementation<"block"> = {};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  nodeRef.value?.focus?.(anchor);
}

defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    v-if="block"
    ref="blockRef"
    class="group/block relative select-none rounded transition-colors duration-150"
    :class="[
      isSelected ? 'bg-orange-400/20' : '',
      isPage ? 'h-[30px] cursor-pointer' : '',
      isPage && !isSelected ? 'hover:bg-gray-100' : '',
    ]"
    :data-suppress-drag="isPage ? 'select' : undefined"
    data-contextmenu-items="space.navigate.open"
    @click="() => isPage && canvas.goToNode(node!)"
  >
    <!-- Page -->
    <div v-if="node && block.type == BlockType.PAGE" class="flex h-[30px] flex-row items-center">
      <NodeReference ref="nodeRef" size="large" is-underline is-light :node="node" :tx="() => connection.tx" />
    </div>
    <!-- Inline definition -->
    <div v-else-if="node">
      <!-- Choice -->
      <Choice
        v-if="block.type == BlockType.CHOICE"
        id="choice"
        ref="nodeRef"
        class=""
        :prepared-connection="preparedConnection"
        :node-ptr="nodePtr"
        is-minimal
        is-inline
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      />
      <Flow
        v-else-if="block.type == BlockType.FLOW"
        id="flow"
        ref="nodeRef"
        class="h-[400px]"
        v-bind="state.getChildState('flow')"
        :node-ptr="nodePtr"
        :prepared-connection="preparedConnection"
        is-minimal
        is-inline
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      />
      <Database
        v-else-if="block.type == BlockType.DATABASE"
        id="database"
        ref="nodeRef"
        v-bind="state.getChildState('database')"
        :node-ptr="nodePtr"
        :container-gutter-width="containerGutterWidth"
        is-minimal
        is-inline
        is-input
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="blockPtr" :connection="connection" />
</template>
