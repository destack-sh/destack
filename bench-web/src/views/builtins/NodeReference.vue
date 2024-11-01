<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import { getNodeType } from "@/language/node";
import { ObjectType, PROPERTY_ENUM_BY_TYPE, Variant, ViewType, type AnyNodeData } from "@/proto/wire";
import { describeNode, isNode } from "@/proto/wiring";
import type { Connection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/ui/icon";
import type { PopoverInfoIn } from "@/ui/popover";
import { getNodeColor } from "@/ui/style";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, ref } from "vue";

const props = defineProps<{
  node: AnyNodeData;
  connection: Connection<"get" | "search", any>;
  isInput?: boolean;
}>();

const inputRef = ref<HTMLInputElement | null>(null);
const nodeType = computed(() => getNodeType(props.node));
const nodeTypeName = computed(() => (nodeType.value != null ? toCamelName(ObjectType, nodeType.value) : "???"));
const nodeProperties = computed(() => (nodeType.value != null ? PROPERTY_ENUM_BY_TYPE[nodeType.value] : null));
const hasName = computed(() => nodeProperties.value != null && "name" in nodeProperties.value);
const name = computed(() => (props.node as any).name);
</script>
<template>
  <div
    :data-node-id="node.id"
    :data-node-ck="(node as any).ck"
    :data-node-type="node.metatype"
    class="flex flex-row items-center"
  >
    <!-- NOTE :UX: hover preview for node references (files, pages, databases, ...) :NodePreviews -->
    <!-- Icon -->
    <IconInline
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.ICON,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: getNodeIcon(node!), isInput: true },
          isEnabled: isInput && nodeProperties != null && 'icon' in nodeProperties,
          onApply: (newIcon) => {
            if (!isNode(node)) throw new Error(`unexpected node: ${describeNode(node)}`);
            connection.tx.update(node, { icon: newIcon });
          },
        })
      "
      v-bind="getNodeIcon(node)"
      :color="getNodeColor(node)"
      class="w-5 rounded-sm p-1 text-gray-700 hover:bg-gray-100 data-[popover=true]:bg-gray-100"
      :class="isInput ? 'cursor-pointer' : ''"
    />
    <!-- Name (editable & has name) -->
    <NativeInput
      id="name"
      ref="nameRef"
      class="ml-2 flex-shrink-0 font-medium text-gray-700 transition-colors duration-150"
      :is-input="hasName && isInput"
      :value-type="NAME_TYPE"
      :variant="Variant.STEALTH"
      :model-value="(node as any).name ?? (node as any).title ?? nodeTypeName"
      @update:model-value="
        (newValue) => isNode(node) && connection.tx.update(node, { name: newValue as string }, { debounce: 'long' })
      "
    />
  </div>
</template>
