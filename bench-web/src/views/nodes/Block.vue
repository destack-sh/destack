<script lang="ts" setup>
import { CANVAS_BLOCK_TYPES } from "@/language/core/const";
import { BlockType, FieldType, NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Text from "@/views/content/Text.vue";
import Database from "@/views/nodes/Database.vue";
import Flow from "@/views/nodes/Flow.vue";
import FieldList from "@/views/objects/FieldList.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    containerGutterWidth?: number;
  } & Partial<Pick<ViewData, "isMinimal" | "nodePtr">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const blockRef = ref<HTMLElement | null>(null);
const nodeRefRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

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

const actions: Partial<ActionMapImplementation<"space">> & ActionMapImplementation<"block"> = {
  // space
  "space.edit.rename": {
    action: () => {
      nextTick(() => nodeRefRef.value?.focusIdentifier());
    },
  },
};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  // TODO :Incomplete: focus/navigate nodes and subnodes (Block/Page) :Navigation
  return false;
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
      isPage ? 'cursor-pointer' : '',
      isPage && !isSelected ? 'hover:bg-gray-100' : '',
    ]"
    :data-suppress-drag="isPage ? 'select' : undefined"
    data-contextmenu-items="space.navigate.open"
    @click="() => isPage && canvas.goToNode(block!)"
  >
    <!-- Header -->
    <div v-if="node" class="flex flex-row items-center rounded-t px-1 py-1">
      <NodeReference
        ref="nodeRefRef"
        :size="hasCanvas ? 'large' : 'regular'"
        :isUnderline="block.type == BlockType.PAGE"
        :node="node"
        :is-input="block.type != BlockType.PAGE"
        :tx="() => connection.tx"
      />
      <!-- Open in its own page -->
      <button
        v-if="hasCanvas"
        class="ml-1.5 rounded px-1 py-0.5 text-base text-gray-400 opacity-0 transition-opacity duration-75 hover:bg-gray-100 hover:text-gray-700 group-focus-within/block-line:opacity-100 group-hover/block-line:opacity-100 group-hover/block:opacity-100"
        @click="() => canvas.goToNode(block!)"
      >
        <i class="fas fa-arrow-up-right" />
      </button>
    </div>
    <!-- Body -->
    <!-- TODO :Incomplete: TextBlock.text is intended to be a single line only
       (but would need to make multi-block navigation/editing better before we enforce this) -->
    <Text
      v-if="block.type == BlockType.PARAGRAPH"
      id="text"
      ref="textRef"
      is-input
      is-minimal
      class="px-1 py-1"
      :model-value="block.text"
      v-bind="state.getChildState('text')"
      @update:model-value="(newText) => connection.tx.update(block!, { text: newText }, { debounce: 'long' })"
    />
    <div v-else-if="block.type != BlockType.PAGE" class="rounded-b border-gray-200 pb-1">
      <!-- Types -->
      <FieldList
        v-if="[BlockType.CHOICE].includes(block.type)"
        id="type"
        :node="block"
        :prepared-connection="preparedConnection"
        :node-ptr="nodePtr"
        :field-type="block.type == BlockType.CHOICE ? FieldType.OPTION : FieldType.MEMBER"
      />
      <!-- Runnable -->
      <template v-if="block.type == BlockType.FLOW">
        <!-- Signature -->
        <div class="flex flex-row flex-wrap items-center gap-x-2 gap-y-1">
          <FieldList
            id="type.input"
            class=""
            :node="block"
            :prepared-connection="preparedConnection"
            :node-ptr="nodePtr"
            :field-type="FieldType.INPUT"
          />
          <i
            v-if="fields?.some((f) => f.type == FieldType.INPUT || f.type == FieldType.OUTPUT)"
            class="fas fa-arrow-right-long text-base text-gray-400"
          />
          <FieldList
            id="type.output"
            class=""
            :node="block"
            :prepared-connection="preparedConnection"
            :node-ptr="nodePtr"
            :field-type="FieldType.OUTPUT"
          />
          <FieldList
            id="type.input"
            class="ml-auto"
            :node="block"
            :prepared-connection="preparedConnection"
            :node-ptr="nodePtr"
            :field-type="FieldType.VARIABLE"
          />
        </div>
        <!-- Flow -->
        <Flow
          v-if="block.type == BlockType.FLOW"
          id="flow"
          class="mt-2 h-[400px]"
          v-bind="state.getChildState('flow')"
          :node-ptr="nodePtr"
          is-minimal
          :prepared-connection="preparedConnection"
        />
      </template>
      <Database
        v-if="block.type == BlockType.DATABASE"
        id="database"
        v-bind="state.getChildState('database')"
        :node-ptr="nodePtr"
        is-minimal
        :container-gutter-width="containerGutterWidth"
        is-input
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="blockPtr" :connection="connection" />
</template>
