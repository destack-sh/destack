<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { NAME_TYPE, TITLE_TYPE } from "@/language/field";
import { Transaction } from "@/language/transaction";
import { AnyNodeData, NodeType, PROPERTY_ENUM_BY_TYPE } from "@/proto/wire";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { PopoverInfoIn } from "@/ui/popover";
import { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { MaybeElement } from "@vueuse/core";
import { computed, Ref, ref } from "vue";

const props = defineProps<{
  size: "regular" | "large" | "title";
  node: AnyNodeData;
  tx?: () => Transaction;
  isUnderline?: boolean;
  isInput?: boolean;
  isLight?: boolean;
  isIconLight?: boolean;
  isMinimal?: boolean;
  maxWidth?: number;
}>();
const iconRef = ref<InstanceType<typeof Icon> | null>(null);
const identifierRef = ref<InstanceType<typeof NativeInput> | null>(null);
const nodeType = computed(() => props.node.metatype as unknown as NodeType);
const nodeTypeName = computed(() => toCamelName(NodeType, nodeType.value));
const nodeProperties = computed(() => (nodeType.value != null ? PROPERTY_ENUM_BY_TYPE[nodeType.value] : null));
const identifierKind = computed(() => {
  if (nodeProperties.value == null) return null;
  else if ("name" in nodeProperties.value) return "name";
  else if ("title" in nodeProperties.value) return "title";
  else return null;
});
const identifier: Ref<string | undefined> = computed(() => (props.node as any)?.[identifierKind.value!]);

// :NodeReferenceStyle
const iconClass = computed(() => [
  props.size == "regular" ? "w-5" : "",
  props.size == "large" ? "w-5 text-base" : "",
  props.size == "title" ? "w-8 text-2xl" : "",
]);
const identifierClass = computed(() => [
  props.size == "regular" ? ["ml-1", props.isLight ? "" : "font-medium"] : "",
  props.size == "large" ? ["ml-1.5 text-xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.size == "title" ? ["ml-1.5 text-3xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.isUnderline ? "underline decoration-gray-300 underline-offset-3" : "",
]);
const metadataClass = computed(() => [
  props.size == "regular" ? "ml-1" : "",
  props.size == "large" ? "ml-1.5" : "",
  props.size == "title" ? "ml-1.5" : "",
]);
const identifierWidthMax = computed(() => {
  if (props.maxWidth != null) return props.maxWidth;
  else if (props.size == "title") return 600;
  else if (props.size == "large") return 400;
  else return 300;
});

function getTx() {
  if (props.tx == null) {
    throw new Error("no transaction provided");
  }
  return props.tx();
}

defineExpose({
  focusIcon: () => focusInElement(iconRef.value!),
  focusIdentifier: () => focusInElement(identifierRef.value as MaybeElement),
});
</script>
<template>
  <div class="flex flex-row items-center">
    <!-- NOTE :Incomplete: support :DelegateNodes (also with base nodes, like Records with icon from their Block) -->
    <IconInline
      ref="iconRef"
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: (node as any)!.icon, isInput: true },
          isEnabled: isInput,
          onApply: (newIcon) => getTx().update(node!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(node)"
      class="rounded text-center data-[popover=true]:bg-gray-100"
      :class="[
        ...iconClass,
        isInput ? 'hover:cursor-pointer hover:bg-gray-100' : '',
        isIconLight ? '' : 'text-gray-700',
      ]"
    />
    <!-- Identifier -->
    <NativeInput
      v-if="isInput"
      id="identifier"
      ref="identifierRef"
      class="flex-shrink-0 rounded text-gray-900 transition-colors duration-150 focus-within:bg-gray-100 hover:bg-gray-100"
      :style="{ maxWidth: `${identifierWidthMax}px` }"
      :class="identifierClass"
      :placeholder="nodeTypeName"
      is-input
      :value-type="identifierKind == 'name' ? NAME_TYPE : TITLE_TYPE"
      is-minimal
      :model-value="identifier"
      @update:model-value="
        (newValue) => getTx().update(node!, { [identifierKind!]: newValue as string }, { debounce: 'long' })
      "
    />
    <span v-else :class="identifierClass" class="truncate" :style="{ maxWidth: `${identifierWidthMax}px` }">
      {{ identifier }}
    </span>
    <!-- Metadata -->
    <NodeMetadata v-if="!isMinimal" :size="size" :node="node" :is-light="isLight" :class="metadataClass" />
  </div>
</template>
