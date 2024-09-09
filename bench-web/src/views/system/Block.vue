<script lang="ts" setup>
import { BLOCK_CONTEXT_ACTIONS } from "@/language/block";
import { NAME_CONSTRAINT, PAGE_BLOCK_TYPES, RUNNABLE_BLOCK_TYPES, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, makeTypeInfo, resolveType, type TypeIdentity } from "@/language/field";
import { isGeneratedNodeName } from "@/language/node";
import { isRunnable } from "@/language/session";
import { packValue, unpackValue } from "@/language/value";
import {
  BenchType,
  BlockType,
  FieldZone,
  NodeReferenceData,
  NodeType,
  Struct as ProtoStruct,
  TypeInfoData,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { onMouseReleasedOnce } from "@/ui/layout";
import { menuActionsLike, pushPopover, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import type { TooltipInfo } from "@/ui/tooltip";
import { getNativeConstraintProps, guardNativeNameInput } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import Value from "@/views/content/Value.vue";
import Flow from "@/views/system/Flow.vue";
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

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);

const runnable = computed(() => block.value != null && isRunnable(block.value, pkgGraph, fields.value));
const isGeneratedName = computed(
  () => block.value != null && isGeneratedNodeName(block.value.metatype as unknown as NodeType, block.value.name),
);
const isQuasiAnonymous = computed(
  () =>
    (block.value?.type == BlockType.TEXT && !hasFunctionFields.value) ||
    (isGeneratedName.value &&
      ((block.value?.type == BlockType.CODE && !hasFunctionFields.value) || block.value?.type == BlockType.VALUE)),
);
const hasText = computed(() => block.value?.text != null);
const hasFunctionFields = computed(
  () =>
    RUNNABLE_BLOCK_TYPES.includes(block.value?.type!) &&
    fields.value.some((f) => f.zone == FieldZone.INPUT || f.zone == FieldZone.OUTPUT),
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
    class="group/block relative rounded"
    :class="[
      borderless || variant == Variant.STEALTH ? '' : 'border',
      nodePtr?.id == inspectionPtr?.id ? 'border-primary-900' : ['border-gray-200 px-2 py-1.5 hover:border-gray-200'],
    ]" 
  >
    <!-- Header -->
    <!-- NOTE :UX: revamp block to indicate all its states/properties better, hide header if not needed, ... -->
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
              props: { modelValue: block!.icon, isInput: true },
              onApply: (newIcon) => pkgConnection.tx.update(block!, { icon: newIcon }),
            })
          "
          v-bind="getNodeIcon(block)"
          class="w-5 rounded py-0.5 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
          :class="isQuasiAnonymous ? 'text-gray-400' : 'text-gray-700'"
        />
        <input
          ref="nameRef"
          type="text"
          class="w-fit min-w-fit max-w-fit rounded border-0 px-1 outline-none ring-0 transition-colors duration-75 hover:bg-gray-100 focus:ring-0"
          :class="[isQuasiAnonymous ? 'px-0.5 text-gray-400' : 'ml-0.5 px-1 font-medium']"
          data-suppress-drag="true"
          spellcheck="false"
          :value="block.name"
          :size="block.name.length + 3"
          v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
          @input="
            guardNativeNameInput($event, block!.name, (newValue) =>
              pkgConnection.tx.update(block!, { name: newValue }, { debounce: 'long' }),
            )
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
          <!-- Open as page -->
          <button
            v-if="PAGE_BLOCK_TYPES.includes(block.type)"
            class="stext-gray-400 flex-shrink-0 rounded px-0.5 hover:bg-gray-100 hover:text-primary-900"
            @click="canvas.goToNode(block, { ifPresent: 'upsertAndFocus' })"
          >
            <i class="fas fa-magnifying-glass-plus" />
          </button>
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
                  createField(pkgConnection.tx, pkgGraph, {
                    anchor: 'inside',
                    target: block!,
                    field: { kind: TypeKind.LITERAL, zone: FieldZone.OPTION },
                  });
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
                        createField(pkgConnection.tx, pkgGraph, { anchor: 'inside', target: block!, field: typeInfo });
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
                items: menuActionsLike(BLOCK_CONTEXT_ACTIONS, { context: { triggerNode: nodePtr } }),
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
        v-if="block.type == BlockType.VALUE"
        :value-type="
          block.valueType != null
            ? (resolveType(block.valueType, pkgGraph) as TypeInfoData) /* close enough */
            : undefined
        "
        :model-value="
          // unfortunate :ProtoStructMapping for every change
          block!.valueType == null
            ? undefined
            : unpackValue(block?.valuePacked == null ? null : ProtoStruct.toJson(block.valuePacked), block!.valueType, {
                graph: pkgGraph,
                unwrapScalar: true,
                recurseValueObject: false,
              })
        "
        @update:model-value="
          (newValue) => {
            const valuePacked = packValue(newValue, block?.valueType!, {
              graph: pkgGraph,
              wrapScalar: true,
              recurseValueObject: false,
            });
            if (valuePacked != null) {
              pkgConnection.tx.update(
                block!,
                { valuePacked: ProtoStruct.fromJson(valuePacked) },
                { debounce: 'short' },
              );
            } else {
              pkgConnection.tx.update(block!, { valuePacked: undefined }, { debounce: 'short' });
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
        :node-ptr="props.nodePtr"
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
      <Flow
        v-if="block.type == BlockType.FLOW"
        class="h-[400px]"
        :node-ptr="props.nodePtr"
        :self="props.self"
        :variant="Variant.COMPACT"
        :prepared-connection="pkgGetConnection"
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="pkgConnection" />
</template>
