<script lang="ts" setup>
import {
  BenchType,
  BlockType,
  FieldZone,
  NodeReferenceData,
  NodeType,
  Struct as ProtoStruct,
  TypeInfoData,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/system/action";
import type { PreparedGetConnection } from "@/system/connection";
import { useGetConnection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/system/icon";
import { RUNNABLE_BLOCK_TYPES, TYPE_BLOCK_TYPES, createField, isGeneratedNodeName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { makeTypeInfo, packValue, resolveType, unpackValue, type TypeIdentity } from "@/system/value";
import { onMouseReleasedOnce } from "@/utils/layout";
import { menuActionsLike, pushPopover, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/utils/menu";
import type { TooltipInfo } from "@/utils/tooltip";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import Value from "@/views/content/Value.vue";
import Type from "@/views/system/Type.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    preparedConnection?: PreparedGetConnection;
    borderless?: boolean;
  } & Pick<ViewData, "variant" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const blockRef = ref<HTMLElement | null>(null);
const nameRef = ref<HTMLElement | null>(null);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const pkgGetConnection =
  props.preparedConnection ??
  useGetConnection(
    { name: `block.${nodePtr.value.id}` },
    computed(() => ({
      roots: [nodePtr.value],
      options: { descendantTypes: [NodeType.FIELD, NodeType.VIEW, NodeType.STEP, NodeType.TRIGGER] },
      isEnabled: nodePtr.value != null,
    })),
  );
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);

const isRunnable = computed(() => RUNNABLE_BLOCK_TYPES.includes(block.value?.type!));
const isGeneratedName = computed(
  () => block.value != null && isGeneratedNodeName(block.value.metatype as unknown as NodeType, block.value.name),
);
const isThinTextWrapper = computed(() => isGeneratedName.value && block.value?.type == BlockType.TEXT);
const hasText = computed(() => block.value?.text != null);
const hasFunctionFields = computed(
  () => isRunnable.value && fields.value.some((f) => f.zone == FieldZone.INPUT || f.zone == FieldZone.OUTPUT),
);
const forceShowText: Ref<boolean> = ref(false);

//
// Interaction
//

const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"block"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => nameRef.value!.focus());
    },
  },
  // block
  "block.edit.isPage": {
    isChecked: () => block.value?.isPage ?? false,
    action: () => pkgConnection.tx.update(block.value!, { isPage: !block.value!.isPage }, { debounce: "tick" }),
  },
  "block.edit.isProtocol": {
    isChecked: () => block.value?.isProtocol ?? false,
    action: () => pkgConnection.tx.update(block.value!, { isProtocol: !block.value!.isProtocol }, { debounce: "tick" }),
  },
  "block.edit.isTemplate": {
    isChecked: () => block.value?.isTemplate ?? false,
    action: () => pkgConnection.tx.update(block.value!, { isTemplate: !block.value!.isTemplate }, { debounce: "tick" }),
  },
  "block.edit.isMaterialized": {
    isChecked: () => block.value?.isMaterialized ?? false,
    action: () =>
      pkgConnection.tx.update(block.value!, { isMaterialized: !block.value!.isMaterialized }, { debounce: "tick" }),
  },
  "block.edit.isPaused": {
    isChecked: () => block.value?.isPaused ?? false,
    action: () => pkgConnection.tx.update(block.value!, { isPaused: !block.value!.isPaused }, { debounce: "tick" }),
  },
};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  // TODO :Incomplete: focus/navigate nodes and subnodes (Block/Page) :Navigation
  return false;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH], actions, focus });
</script>
<template>
  <div
    v-if="block"
    ref="blockRef"
    class="group/block relative rounded bg-white"
    :class="[
      borderless ? '' : 'border',
      nodePtr?.id == inspectionPtr?.id
        ? 'border-primary-900'
        : [variant != Variant.STEALTH ? 'border-gray-200 px-2 py-1.5' : 'border-transparent', 'hover:border-gray-200'],
    ]"
  >
    <!-- Header -->
    <!-- TODO :UX: indicate Block.isPage/isProtocol/isTemplate -->
    <div class="flex flex-row">
      <!-- Icon/Name (also drag handle if container is not already draggable) -->
      <div
        @mousedown="
          () =>
            blockRef!.draggable ||
            ((blockRef!.draggable = true), onMouseReleasedOnce(() => (blockRef!.draggable = false)))
        "
      >
        <IconInline
          v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
          v-menu="
            (): PopoverInfoIn => ({
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: block!.icon },
              onApply: (newIcon) => pkgConnection.tx.update(block!, { icon: newIcon }),
            })
          "
          v-bind="getNodeIcon(block)"
          class="w-5 rounded py-0.5 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
          :class="isThinTextWrapper ? 'text-gray-500' : 'text-gray-700'"
        />
        <input
          ref="nameRef"
          class="w-fit min-w-fit max-w-fit rounded border-0 px-1 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
          :class="[isThinTextWrapper ? 'px-0.5 text-gray-500' : 'ml-0.5 px-1 font-medium']"
          spellcheck="false"
          :value="block.name"
          :size="Math.max(block.name.length, 5)"
          @input="
            (event) => {
              pkgConnection.tx.update(block!, { name: (event.target as HTMLInputElement).value }, { debounce: 'long' });
            }
          "
        />
      </div>
      <!-- Tags, triggers, roles, queries, etc. -->
      <div class="ml-auto pl-2 pr-0.5">
        <!-- Quick actions -->
        <span
          class="flex flex-row gap-x-0.5 transition-colors duration-75"
          :class="[
            nodePtr?.id == inspectionPtr?.id
              ? 'text-gray-400'
              : [
                  variant != Variant.STEALTH ? '' : 'opacity-0 group-hover/block:opacity-100',
                  'text-gray-300  group-hover/block:text-gray-400',
                ],
          ]"
        >
          <!-- Add/edit text -->
          <button
            v-if="!hasText && block.type != BlockType.TEXT"
            class="rounded hover:bg-gray-100 hover:text-primary-900"
            @click="
              () => {
                forceShowText = true;
                nextTick(() => textRef?.focus?.('center'));
              }
            "
          >
            <i class="fas fa-text w-5 text-center" />
          </button>
          <!-- Quick add -->
          <button
            v-if="TYPE_BLOCK_TYPES.includes(block.type) || RUNNABLE_BLOCK_TYPES.includes(block.type)"
            class="rounded hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
            @click="
              (e) => {
                if (block!.type == BlockType.CHOICE) {
                  createField(pkgConnection.tx, pkgGraph, 'inside', block!, { zone: FieldZone.OPTION });
                } else {
                  const button = (e.target as HTMLElement).closest('button')!;
                  pushPopover({
                    trigger: button,
                    reference: button,
                    info: {
                      component: ViewType.PICKER,
                      placement: 'bottom-left',
                      offset: 'referenceWidth',
                      props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
                      onApply: (typeInfo: TypeIdentity) => {
                        createField(pkgConnection.tx, pkgGraph, 'inside', block!, typeInfo);
                      },
                    },
                  });
                }
              }
            "
          >
            <i class="fas fa-plus w-5 text-center" />
          </button>
          <!-- Menu -->
          <button
            v-menu="
              (): PopoverInfo => ({
                kind: 'menu',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                items: menuActionsLike(
                  [
                    'common.edit.rename',
                    'common.edit.morph',
                    'common.edit.move',
                    'common.edit.duplicate',
                    'common.edit.archive',
                    'common.edit.delete',
                    'message.handle.startThread',
                  ],
                  { context: { triggerNode: nodePtr } },
                ),
              })
            "
            class="rounded hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
          >
            <i class="fas fa-ellipsis-v w-5 text-center" />
          </button>
        </span>
      </div>
      <!-- ... -->
    </div>
    <!-- Body -->
    <div class="flex flex-col gap-y-1.5 py-1">
      <!-- Variable(s) ... -->
      <Value
        v-if="block.type == BlockType.VARIABLE"
        :value-type="
          block.valueType != null
            ? (resolveType(block.valueType, pkgGraph) as TypeInfoData) /* close enough */
            : undefined
        "
        :model-value="
          // unfortunate :ProtoStructMapping for every change
          block!.valueType == null
            ? undefined
            : unpackValue(
                {
                  valuePacked: block?.valuePacked == null ? null : ProtoStruct.toJson(block.valuePacked),
                  secretValuePacked:
                    block?.secretValuePacked == null ? null : ProtoStruct.toJson(block.secretValuePacked),
                },
                block!.valueType,
                pkgGraph,
              )
        "
        @update:model-value="
          (newValue) => {
            const packed = packValue(newValue, block?.valueType!, pkgGraph);
            if (packed.secretValuePacked != null) {
              pkgConnection.tx.update(
                block!,
                {
                  valuePacked: ProtoStruct.fromJson(packed.valuePacked),
                  secretValuePacked: ProtoStruct.fromJson(packed.secretValuePacked),
                },
                { debounce: 'short' },
              );
            } else if (packed.valuePacked != null) {
              pkgConnection.tx.update(
                block!,
                { valuePacked: ProtoStruct.fromJson(packed.valuePacked) },
                { debounce: 'short' },
              );
            } else {
              pkgConnection.tx.update(
                block!,
                { valuePacked: undefined, secretValuePacked: undefined },
                { debounce: 'short' },
              );
            }
          }
        "
      />
      <Type
        v-if="
          [BlockType.CLASS, BlockType.CHOICE, BlockType.SIGNAL].includes(block.type) ||
          (RUNNABLE_BLOCK_TYPES.includes(block.type) && hasFunctionFields)
        "
        :node="block"
        :prepared-connection="pkgGetConnection"
        :node-ptr="nodePtr"
      />
      <!-- TODO :UX: Text/Code empty states? -->
      <Text
        v-if="block.type == BlockType.TEXT || block.text != null || forceShowText"
        ref="textRef"
        is-input
        :variant="Variant.STEALTH"
        :model-value="block.text"
        @update:model-value="(newText) => pkgConnection.tx.update(block!, { text: newText }, { debounce: 'long' })"
      />
      <Code
        v-if="block.type == BlockType.CODE"
        is-input
        :model-value="block.code"
        @update:model-value="(newCode) => pkgConnection.tx.update(block!, { code: newCode }, { debounce: 'long' })"
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
