<script lang="ts" setup>
import {
  BenchType,
  BlockType,
  ColorShade,
  ColorType,
  FieldZone,
  NodeReferenceData,
  NodeType,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/system/action";
import type { PreparedGetConnection } from "@/system/connection";
import { useGetConnection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/system/icon";
import { RUNNABLE_BLOCK_TYPES, TYPE_BLOCK_TYPES, createField, isGeneratedNodeName, toCamelName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { onMouseReleasedOnce } from "@/utils/layout";
import { pushPopover, menuActionsLike, type PopoverInfo, type PopoverInfoIn } from "@/utils/menu";
import type { TooltipInfo } from "@/utils/tooltip";
import Type from "@/views/system/Type.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { makeTypeInfo, packValue, unpackValue, type TypeIdentity } from "@/system/value";
import Value from "@/views/content/Value.vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr"
  >
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
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isPage: !block.value!.isPage }),
  },
  "block.edit.isProtocol": {
    isChecked: () => block.value?.isProtocol ?? false,
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isProtocol: !block.value!.isProtocol }),
  },
  "block.edit.isTemplate": {
    isChecked: () => block.value?.isTemplate ?? false,
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isTemplate: !block.value!.isTemplate }),
  },
};

// focus
function focus(anchor: FocusAnchor | NodeReferenceData) {
  // TODO :Incomplete: focus/navigate nodes and subnodes (Block/Page) :Navigation
  return false;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH], actions, focus });
</script>
<template>
  <div
    ref="blockRef"
    v-if="block"
    class="group/block relative rounded border bg-white px-2 py-1.5"
    :class="[
      nodePtr?.id == inspectionPtr?.id
        ? 'border-primary-900'
        : [variant != Variant.STEALTH ? 'border-gray-200' : 'border-transparent', 'hover:border-gray-300'],
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
          v-bind="getNodeIcon(block)"
          class="w-5 rounded border border-transparent py-0.5 hover:cursor-pointer hover:bg-gray-100 data-[menu=true]:border-primary-900 data-[menu=true]:bg-gray-100"
          :class="isThinTextWrapper ? 'text-gray-500' : 'text-gray-700'"
          v-tooltip="{ small: true, text: `Change icon` }"
          v-menu="
            (): PopoverInfoIn => ({
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: block!.icon },
              onApply: (newIcon) => pkgConnection.tx.update(block!, { icon: newIcon }),
            })
          "
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
              pkgConnection.tx.updateDebounced(block!, { name: (event.target as HTMLInputElement).value });
            }
          "
        />
      </div>
      <!-- Tags, triggers, roles, queries, etc. -->
      <div class="ml-auto pl-2 pr-0.5">
        <!-- Quick actions -->
        <span
          class="flex flex-row gap-x-0.5"
          :class="[
            nodePtr?.id == inspectionPtr?.id
              ? 'text-gray-400'
              : [
                  variant != Variant.STEALTH ? '' : 'opacity-0  group-hover/block:opacity-100',
                  'text-gray-300  group-hover/block:text-gray-400',
                ],
          ]"
        >
          <!-- Add/edit text -->
          <button
            v-if="!hasText && block.type != BlockType.TEXT"
            class="t rounded px-1 hover:bg-gray-100 hover:text-primary-900"
            @click="
              () => {
                forceShowText = true;
                nextTick(() => textRef?.focus?.('center'));
              }
            "
          >
            <i class="fas fa-text" />
          </button>
          <!-- Quick add -->
          <button
            v-if="TYPE_BLOCK_TYPES.includes(block.type) || RUNNABLE_BLOCK_TYPES.includes(block.type)"
            class="rounded border border-transparent px-0.5 hover:bg-gray-100 hover:text-primary-900 data-[menu=true]:border-primary-900 data-[menu=true]:bg-gray-100 data-[menu=true]:text-primary-900"
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
            <i class="fas fa-plus" />
          </button>
          <!-- Menu -->
          <button
            class="rounded border border-transparent px-2 hover:bg-gray-100 hover:text-primary-900 data-[menu=true]:border-primary-900 data-[menu=true]:bg-gray-100 data-[menu=true]:text-primary-900"
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
                    'common.edit.delete',
                    'block.*',
                  ],
                  { context: { triggerNode: nodePtr } },
                ),
              })
            "
          >
            <i class="fas fa-ellipsis-v" />
          </button>
        </span>
      </div>
      <!-- ... -->
    </div>
    <!-- Body -->
    <div class="flex flex-col gap-y-1.5 py-0.5">
      <!-- Variable(s) ... -->
      <Value
        v-if="block.type == BlockType.VARIABLE"
        :value-type="block.builtinBase"
        :model-value="unpackValue(block!, block.builtinBase!, pkgGraph)"
        @update:modelValue="
          (newValue) =>
            pkgConnection.tx.updateDebounced(block!, packValue(newValue, block?.builtinBase!, pkgGraph, block!))
        "
      />
      <Type
        v-if="TYPE_BLOCK_TYPES.includes(block.type) || (RUNNABLE_BLOCK_TYPES.includes(block.type) && hasFunctionFields)"
        :node="block"
        :prepared-connection="pkgGetConnection"
        :node-ptr="nodePtr"
      />
      <!-- TODO :UX: Text/Code empty states? -->
      <Text
        ref="textRef"
        v-if="block.type == BlockType.TEXT || block.text != null || forceShowText"
        is-input
        :variant="Variant.STEALTH"
        :model-value="block.text"
        @update:modelValue="(newText) => pkgConnection.tx.updateDebounced(block!, { text: newText })"
      />
      <Code
        v-if="block.type == BlockType.CODE"
        is-input
        :model-value="block.code"
        @update:modelValue="(newCode) => pkgConnection.tx.updateDebounced(block!, { code: newCode })"
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
