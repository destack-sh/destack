<script lang="ts" setup>
import { BLOCK_CONTEXT_ACTIONS } from "@/language/block";
import { PAGE_BLOCK_TYPES, RUNNABLE_BLOCK_TYPES } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import { unpackSubnodeProperty } from "@/language/node";
import { makeEdit } from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  BlockType,
  ColorShade,
  FieldType,
  NodeReferenceData,
  NodeType,
  TypeKind,
  Variant,
  ViewData,
} from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { startDraggingIfAllowed } from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { menuActionsLike, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { getNodeColorHex } from "@/ui/style";
import type { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import Text from "@/views/content/Text.vue";
import Value from "@/views/content/Value.vue";
import Database from "@/views/system/Database.vue";
import Flow from "@/views/system/Flow.vue";
import Type from "@/views/system/Type.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = 32;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
  } & Pick<ViewData, "variant" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const blockRef = ref<HTMLElement | null>(null);
const nameRef = ref<InstanceType<typeof NativeInput> | null>(null);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);

const hasFunctionFields = computed(
  () =>
    RUNNABLE_BLOCK_TYPES.includes(block.value?.type!) &&
    fields.value.some((f) => f.type == FieldType.INPUT || f.type == FieldType.OUTPUT),
);
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));

//
// Interaction
//

const valueType = computed(() => {
  if (block.value?.type != BlockType.VALUE) {
    return undefined;
  }
  return unpackSubnodeProperty(NodeType.BLOCK, BlockType.VALUE, block.value.subnodePacked, "valueType");
});
const value = computed(() => {
  if (block.value?.type != BlockType.VALUE || valueType.value == null) {
    return undefined;
  }
  const valuePacked = unpackSubnodeProperty(NodeType.BLOCK, BlockType.VALUE, block.value.subnodePacked, "valuePacked");
  if (valueType == null) {
    return undefined;
  } else if (valueType.value.kind == TypeKind.OBJECT) {
    return valuePacked;
  } else {
    return unpackValue(valuePacked!, valueType.value, {
      graph: pkgGraph,
      wrapScalar: true,
      recurseCustomObject: false,
    });
  }
});
function updateValue(value: any) {
  if (block.value?.type != BlockType.VALUE) throw new Error(`no value block`);
  const valueType = unpackSubnodeProperty(NodeType.BLOCK, BlockType.VALUE, block.value.subnodePacked, "valueType");
  const valuePacked =
    valueType?.kind == TypeKind.OBJECT
      ? value
      : packValue(value, valueType!, { graph: pkgGraph, wrapScalar: true, recurseCustomObject: false });
  if (valuePacked != null) {
    pkgConnection.tx.update(
      block.value,
      makeEdit(block.value, { metatype: NodeType.BLOCK, type: BlockType.VALUE, subnode: { valuePacked } }),
      { debounce: "short" },
    );
  } else {
    pkgConnection.tx.update(
      block.value,
      makeEdit(block.value, { metatype: NodeType.BLOCK, type: BlockType.VALUE, subnode: { valuePacked: undefined } }),
      { debounce: "short" },
    );
  }
}

const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"block"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => focusInElement(nameRef.value!));
    },
  },
  "common.navigate.open": (action, ctx) => {
    if (block.value == null) return false;
    canvas.goToNode(block.value, { where: "bestFrame" });
  },
  "common.navigate.openInPage": (action, ctx) => {
    if (block.value == null) return false;
    canvas.goToNode(block.value, { where: "bestFrame", preferPage: true });
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
    class="group/block relative select-none rounded"
    :class="[isInspected ? 'border-primary-700' : 'border-gray-200']"
    :style="{}"
  >
    <!-- Header -->
    <div
      v-if="block.type != BlockType.TEXT && block.type"
      class="flex flex-row items-center rounded-t hover:cursor-grab"
      :style="{
        height: `${HEADER_HEIGHT}px`,
      }"
    >
      <!-- Icon -->
      <div class="ml-1 flex flex-row items-center rounded px-0.5">
        <IconInline
          v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
          v-menu="
            (): PopoverInfoIn => ({
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: block!.icon, isInput: true },
              onApply: (newIcon) => pkgConnection.tx.update(block!, { icon: newIcon }),
            })
          "
          v-bind="getNodeIcon(block)"
          :style="{
            color: getNodeColorHex(block, ColorShade.S600),
          }"
          class="w-5 rounded-sm py-0.5 text-gray-700 hover:cursor-pointer"
        />
      </div>
      <!-- Name -->
      <NativeInput
        id="name"
        ref="nameRef"
        class="ml-2 flex-shrink-0 font-medium transition-colors duration-150"
        is-input
        :value-type="NAME_TYPE"
        placeholder="Name..."
        :variant="Variant.STEALTH"
        :model-value="block.name"
        @update:model-value="
          (newValue) => pkgConnection.tx.update(block!, { name: newValue as string }, { debounce: 'long' })
        "
      />
      <!-- Tags, triggers, roles, queries, etc. -->
      <div class="ml-auto pl-2 pr-1.5">
        <!-- Quick actions -->
        <div class="flex flex-row gap-x-1.5">
          <!-- Open -->
          <button
            v-if="PAGE_BLOCK_TYPES.includes(block.type)"
            class="text-gray-400 hover:text-gray-700"
            @click="canvas.goToNode(block!, { where: 'bestFrame' })"
          >
            <i class="fas fa-magnifying-glass-plus" />
          </button>
          <!-- Menu -->
          <button
            v-menu="
              (): PopoverInfo => ({
                kind: 'menu',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                items: menuActionsLike(BLOCK_CONTEXT_ACTIONS, { context: { triggerNode: nodePtr } }),
              })
            "
            class="text-gray-400 hover:text-gray-700"
          >
            <i class="fas fa-ellipsis-v w-5 text-center" />
          </button>
        </div>
      </div>
      <!-- ... -->
    </div>
    <!-- Body -->
    <Text
      v-if="block.type == BlockType.TEXT"
      id="text"
      ref="textRef"
      is-input
      class="px-0.5"
      :variant="Variant.STEALTH"
      :model-value="unpackSubnodeProperty(NodeType.BLOCK, block.type, block.subnodePacked, 'text')"
      v-bind="state.getChildState('text')"
      @update:model-value="
        (newText) =>
          pkgConnection.tx.update(
            block!,
            makeEdit(block!, { metatype: NodeType.BLOCK, type: BlockType.TEXT, subnode: { text: newText } }),
            { debounce: 'long' },
          )
      "
    />
    <div v-else-if="block.type != BlockType.PAGE" class="rounded-b border-gray-200">
      <!-- Types -->
      <Type
        v-if="[BlockType.CLASS, BlockType.CHOICE, BlockType.MESSAGE].includes(block.type)"
        id="type"
        class="px-1 py-1"
        :node="block"
        :prepared-connection="pkgGetConnection"
        :node-ptr="props.nodePtr"
        :field-type="block.type == BlockType.CHOICE ? FieldType.OPTION : FieldType.MEMBER"
      />
      <!-- Runnable -->
      <template v-if="block.type == BlockType.ACTION || block.type == BlockType.FLOW">
        <!-- Signature -->
        <div class="flex flex-row flex-wrap items-center gap-x-2 gap-y-1 border-gray-200 px-2 pb-2 pt-1">
          <Type
            id="type.input"
            class=""
            :node="block"
            :prepared-connection="pkgGetConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.INPUT"
          />
          <i v-if="hasFunctionFields" class="fas fa-arrow-right-long text-base text-gray-400" />
          <Type
            id="type.output"
            class=""
            :node="block"
            :prepared-connection="pkgGetConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.OUTPUT"
          />
        </div>
        <!-- Action -->
        <Text
          v-if="block.type == BlockType.ACTION"
          id="text"
          ref="textRef"
          is-input
          class=""
          :class="block.type == BlockType.ACTION ? 'px-2' : ''"
          :variant="Variant.STEALTH"
          placeholder="Text..."
          :model-value="unpackSubnodeProperty(NodeType.BLOCK, block.type, block.subnodePacked, 'text')"
          v-bind="state.getChildState('text')"
          @update:model-value="
            (newText) =>
              pkgConnection.tx.update(
                block!,
                makeEdit(block!, { metatype: NodeType.BLOCK, type: BlockType.ACTION, subnode: { text: newText } }),
                { debounce: 'long' },
              )
          "
        />
        <!-- Flow -->
        <Flow
          v-else-if="block.type == BlockType.FLOW"
          id="flow"
          class="h-[400px]"
          v-bind="state.getChildState('flow')"
          :node-ptr="props.nodePtr"
          :variant="Variant.COMPACT"
          :prepared-connection="pkgGetConnection"
        />
      </template>
      <!-- State -->
      <Value
        v-if="block.type == BlockType.VALUE"
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
        :variant="Variant.COMPACT"
        is-input
        :padding-x="6"
        :padding-y="8"
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="pkgConnection" />
</template>
