<script lang="ts" setup>
import { BlockType, NodeReferenceData, NodeType, Variant, ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { useGetConnection } from "@/system/connection";
import type { PreparedGetConnection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { onMouseNotPressedOnce } from "@/utils/layout";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { computed, toRef, type Ref, ref } from "vue";
import type { TooltipInfo } from "@/utils/tooltip";
import type { ActionMapImplementation } from "@/system/action";
import PlainText from "@/views/content/PlainText.vue";
import type { OverlayMenuInfo } from "@/utils/menu";
import Text from "@/views/content/Text.vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "nodePtr"
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

// nocheckin :Incomplete: Block

//
// Interaction
//

const actions: Partial<ActionMapImplementation<"common">> = {
  "common.block.toggleIsPage": {
    isEnabled: () => block.value != null,
    isChecked: () => block.value?.isPage ?? false,
    action: () => pkgConnection.tx.updateDebounced(block.value!, { isPage: !block.value!.isPage }),
  },
  "common.block.toggleIsProtocol": {
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
defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div ref="blockRef" v-if="block" class="group/block px-2 py-1.5">
    <!-- Header -->
    <div>
      <!-- Icon/Name (also drag handle) -->
      <span
        class=""
        @mousedown="
          () =>
            blockRef!.draggable ||
            ((blockRef!.draggable = true), onMouseNotPressedOnce(() => (blockRef!.draggable = false)))
        "
      >
        <IconInline
          v-bind="getNodeIcon(block)"
          class="rounded-md px-0.5 py-0.5 text-gray-600 hover:cursor-pointer hover:bg-primary-100 hover:text-primary-900"
          v-tooltip="
            {
              showDelay: 400,
              hideDelay: 200,
              placement: 'top',
              small: true,
              text: `Change icon (${toCamelName(BlockType, block.type)})`,
            } as TooltipInfo
          "
        />
        <span
          role="button"
          class="ml-1 rounded-md px-0.5 py-0.5 hover:cursor-pointer hover:bg-primary-100 hover:text-primary-900"
          v-menu="
            (): OverlayMenuInfo => ({
              kind: 'component',
              placement: 'inside-top-left',
              referenceOffset: { x: 0, y: -2 },
              fitToContainer: 'width',
              props: { modelValue: block!.name, isInput: true, variant: Variant.STEALTH },
              onApply: (name: string) => {
                pkgConnection.tx.updateDebounced(block!, { name });
              },
              component: PlainText,
            })
          "
        >
          {{ block.name }}
        </span>
      </span>
      <!-- Tags, triggers, roles, queries, etc. -->
    </div>
    <!-- Body -->
    <div class="py-1">
      nocheckin: Body
      <Text
        is-input
        :variant="Variant.STEALTH"
        :model-value="block.text"
        @update:modelValue="
          (newText) => {
            pkgConnection.tx.update(block!, { text: newText });
          }
        "
      />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
