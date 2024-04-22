<script lang="ts" setup>
import { BlockType, NodeReferenceData, NodeType, Variant, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/system/action";
import type { PreparedGetConnection } from "@/system/connection";
import { useGetConnection } from "@/system/connection";
import { isGeneratedNodeName } from "@/system/graph";
import { IconInline, getNodeIcon } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { onMouseNotPressedOnce } from "@/utils/layout";
import type { OverlayMenuInfo } from "@/utils/menu";
import type { TooltipInfo } from "@/utils/tooltip";
import { makeViewId } from "@/views";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import { computed, ref, toRef, type Ref } from "vue";

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
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `block.${nodePtr.value.id}` },
    computed(() => ({
      roots: [nodePtr.value],
      options: { descendantTypes: [NodeType.FIELD, NodeType.VIEW, NodeType.STEP, NodeType.TRIGGER] },
      isEnabled: nodePtr.value != null,
    })),
  );
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isGeneratedName = computed(
  () =>
    block.value != null &&
    isGeneratedNodeName(block.value.metatype as unknown as NodeType, block.value.type, block.value.name),
);
const isThinTextWrapper = computed(() => isGeneratedName.value && block.value?.type == BlockType.TEXT);

//
// Interaction
//

const actions: Partial<ActionMapImplementation<"block">> = {
  "block.edit.isPage": {
    isEnabled: () => block.value != null,
    isChecked: () => block.value?.isPage ?? false,
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isPage: !block.value!.isPage }),
  },
  "block.edit.isProtocol": {
    isChecked: () => block.value?.isProtocol ?? false,
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isProtocol: !block.value!.isProtocol }),
  },
};

// focus
function focus(anchor: FocusAnchor | NodeReferenceData) {
  console.log("Block.focus: nocheckin", anchor);
  return false;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH], actions, focus });
</script>
<template>
  <div
    ref="blockRef"
    v-if="block"
    class="group/block relative rounded bg-white px-2 py-1.5"
    :class="[
      variant != Variant.STEALTH ? 'border' : '',
      nodePtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200 hover:border-gray-400',
    ]"
  >
    <!-- Header -->
    <!-- TODO: :UX: the floating headers are intended to make simple text blocks less obtrusive.. not great yet -->
    <div :class="[isThinTextWrapper ? 'absolute -top-2.5 left-2 bg-white px-0.5' : '']">
      <!-- Icon/Name (also drag handle if container is not already draggable) -->
      <span
        @mousedown="
          () =>
            blockRef!.draggable ||
            ((blockRef!.draggable = true), onMouseNotPressedOnce(() => (blockRef!.draggable = false)))
        "
      >
        <IconInline
          v-bind="getNodeIcon(block)"
          class="w-5 rounded border border-transparent py-0.5 hover:cursor-pointer hover:bg-primary-100 hover:text-primary-900 data-[menu=true]:border-primary-900 data-[menu=true]:bg-primary-100"
          :class="[isThinTextWrapper ? ' text-gray-500' : 'text-gray-700']"
          v-tooltip="
            {
              showDelay: 400,
              hideDelay: 200,
              placement: 'top',
              small: true,
              text: `Change icon (${toCamelName(BlockType, block.type)})`,
            } as TooltipInfo
          "
          v-menu="
            () =>
              ({
                kind: 'component',
                referenceMargin: 4,
                placement: 'bottom',
                component: Icon,
                props: { modelValue: block!.icon, isInline: true },
                onApply: (newIcon) => {
                  pkgConnection.tx.update(block!, { icon: newIcon });
                },
              }) as OverlayMenuInfo
          "
        />
        <span
          role="button"
          class="min-w-fit max-w-fit rounded border-0 px-1 py-0.5 outline-none ring-0 hover:bg-primary-100 hover:text-primary-900 focus:ring-0"
          :class="[isThinTextWrapper ? 'px-0.5 text-gray-500' : 'ml-0.5 px-1 font-semibold']"
          contenteditable
          :value="block.name"
          @input="
            (event) => {
              pkgConnection.tx.updateDebounced(block!, { name: (event.target as HTMLSpanElement).innerText });
            }
          "
        >
          {{ block.name }}
        </span>
      </span>
      <!-- Tags, triggers, roles, queries, etc. -->
      <!-- ... -->
    </div>
    <!-- Body -->
    <div class="py-1">
      <Text
        v-if="block.type == BlockType.TEXT"
        is-input
        :variant="Variant.STEALTH"
        :model-value="block.text"
        @update:modelValue="
          (newText) => {
            pkgConnection.tx.update(block!, { text: newText });
          }
        "
      />
      <Code
        v-else-if="block.type == BlockType.CODE"
        is-input
        :variant="Variant.STEALTH"
        :model-value="block.code"
        @update:modelValue="
          (newCode) => {
            pkgConnection.tx.update(block!, { code: newCode });
          }
        "
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
