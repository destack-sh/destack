<script lang="ts" setup>
import { CANVAS_BLOCK_TYPES } from "@/language/const";
import { unpackSubnodeProperty } from "@/language/node";
import { getTransactionOptionsForType, makeEdit } from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import { BlockType, FieldType, NodeReferenceData, NodeType, TypeKind, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Text from "@/views/content/Text.vue";
import Value from "@/views/content/Value.vue";
import Database from "@/views/system/Database.vue";
import Flow from "@/views/system/Flow.vue";
import FieldList from "@/views/system/FieldList.vue";
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
const NodeReferenceRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const block = graph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const fields = graph.getChildrenRef(block, NodeType.FIELD);

const isPage = computed(() => block.value?.type == BlockType.PAGE);
const hasCanvas = computed(() => CANVAS_BLOCK_TYPES.includes(block.value?.type!));
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => state.isSelected(nodePtr.value));

//
// Interaction
//

const valueType = computed(() => {
  if (block.value?.type != BlockType.VARIABLE) {
    return undefined;
  }
  return unpackSubnodeProperty(NodeType.BLOCK, BlockType.VARIABLE, block.value.subnodePacked, "valueType");
});
const value = computed(() => {
  if (block.value?.type != BlockType.VARIABLE || valueType.value == null) {
    return undefined;
  }
  const valuePacked = unpackSubnodeProperty(NodeType.BLOCK, BlockType.VARIABLE, block.value.subnodePacked, "valuePacked");
  if (valueType == null) {
    return undefined;
  } else if (valueType.value.kind == TypeKind.CUSTOM_OBJECT || valueType.value.kind == TypeKind.PARTIAL_OBJECT) {
    return valuePacked;
  } else {
    return unpackValue(valuePacked!, valueType.value, {
      graph: graph,
      wrapScalar: true,
      recurseCustomObject: false,
    });
  }
});
function updateValue(value: any) {
  if (block.value?.type != BlockType.VARIABLE) throw new Error(`no value block`);
  const valueType = unpackSubnodeProperty(NodeType.BLOCK, BlockType.VARIABLE, block.value.subnodePacked, "valueType");
  const valuePacked =
    valueType?.kind == TypeKind.CUSTOM_OBJECT || valueType?.kind == TypeKind.PARTIAL_OBJECT
      ? value
      : packValue(value, valueType!, { graph: graph, wrapScalar: true, recurseCustomObject: false });
  if (valuePacked != null) {
    connection.tx.update(
      block.value,
      makeEdit(block.value, { metatype: NodeType.BLOCK, type: BlockType.VARIABLE, subnode: { valuePacked } }),
      valueType != null ? getTransactionOptionsForType(valueType) : { debounce: "short" },
    );
  } else {
    connection.tx.update(
      block.value,
      makeEdit(block.value, { metatype: NodeType.BLOCK, type: BlockType.VARIABLE, subnode: { valuePacked: undefined } }),
      valueType != null ? getTransactionOptionsForType(valueType) : { debounce: "short" },
    );
  }
}

const actions: Partial<ActionMapImplementation<"space">> & ActionMapImplementation<"block"> = {
  // space
  "space.edit.rename": {
    action: () => {
      nextTick(() => NodeReferenceRef.value?.focusIdentifier());
    },
  },
  "space.navigate.open": (action, ctx) => {
    if (block.value == null) return false;
    canvas.goToNode(block.value);
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
    @click="() => isPage && canvas.goToNode(block!)"
  >
    <!-- Header -->
    <div v-if="block.type != BlockType.TEXT" class="flex flex-row items-center rounded-t px-1 py-1">
      <NodeReference
        ref="NodeReferenceRef"
        :size="hasCanvas ? 'large' : 'regular'"
        :underline="block.type == BlockType.PAGE"
        :node="block"
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
    <Text
      v-if="block.type == BlockType.TEXT"
      id="text"
      ref="textRef"
      is-input
      is-minimal
      class="px-1 py-1"
      :is-minimal="isMinimal"
      :model-value="block.text"
      v-bind="state.getChildState('text')"
      @update:model-value="(newText) => connection.tx.update(block!, { text: newText }, { debounce: 'long' })"
    />
    <div v-else-if="block.type != BlockType.PAGE" class="rounded-b border-gray-200 pb-1">
      <!-- Types -->
      <FieldList
        v-if="[BlockType.CHOICE, BlockType.MESSAGE].includes(block.type)"
        id="type"
        :node="block"
        :prepared-connection="preparedConnection"
        :node-ptr="props.nodePtr"
        :field-type="block.type == BlockType.CHOICE ? FieldType.OPTION : FieldType.MEMBER"
      />
      <!-- Runnable -->
      <template v-if="block.type == BlockType.ACTION || block.type == BlockType.FLOW">
        <!-- Signature -->
        <div class="flex flex-row flex-wrap items-center gap-x-2 gap-y-1">
          <FieldList
            id="type.input"
            class=""
            :node="block"
            :prepared-connection="preparedConnection"
            :node-ptr="props.nodePtr"
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
            :node-ptr="props.nodePtr"
            :field-type="FieldType.OUTPUT"
          />
          <FieldList
            id="type.input"
            class="ml-auto"
            :node="block"
            :prepared-connection="preparedConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.VARIABLE"
          />
        </div>
        <!-- Flow -->
        <Flow
          v-if="block.type == BlockType.FLOW"
          id="flow"
          class="mt-2 h-[400px]"
          v-bind="state.getChildState('flow')"
          :node-ptr="props.nodePtr"
          is-minimal
          :prepared-connection="preparedConnection"
        />
      </template>
      <!-- State -->
      <Value
        v-if="block.type == BlockType.VARIABLE"
        id="value"
        class="max-h-[320px]"
        :value-type="valueType"
        :model-value="value"
        :size="{ height: 320 }"
        v-bind="state.getChildState('value')"
        @update:model-value="(newValue) => updateValue(newValue)"
      />
      <Database
        v-else-if="block.type == BlockType.DATABASE"
        id="database"
        v-bind="state.getChildState('database')"
        :node-ptr="props.nodePtr"
        is-minimal
        :containerGutterWidth="containerGutterWidth"
        is-input
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
</template>
