<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import { Transaction } from "@/language/transaction";
import { AnyNodeData, NodeType, Variant } from "@/proto/wire";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { PopoverInfoIn } from "@/ui/popover";
import { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, ref } from "vue";

const props = defineProps<{
  size: "regular" | "large" | "title";
  node: AnyNodeData;
  tx?: () => Transaction;
  underline?: boolean;
  light?: boolean;
  isInput?: boolean;
}>();
const iconRef = ref<InstanceType<typeof Icon> | null>(null);
const nameRef = ref<InstanceType<typeof NativeInput> | null>(null);
const nodeTypeName = computed(() => toCamelName(NodeType, props.node.metatype));
const iconClass = computed(() => [
  props.size == "regular" ? "w-5" : "",
  props.size == "large" ? "w-5 text-base" : "",
  props.size == "title" ? "w-8 text-2xl" : "",
]);
const nameClass = computed(() => [
  props.size == "regular" ? ["ml-1", props.light ? "" : "font-medium"] : "",
  props.size == "large" ? ["ml-1.5 text-xl", props.light ? "font-medium" : "font-bold"] : "",
  props.size == "title" ? ["ml-1.5 text-3xl", props.light ? "font-medium" : "font-bold"] : "",
  props.underline ? "underline decoration-gray-300 underline-offset-3" : "",
]);

function getTx() {
  if (props.tx == null) {
    throw new Error("no transaction provided");
  }
  return props.tx();
}

defineExpose({
  focusIcon: () => focusInElement(iconRef.value!),
  focusName: () => focusInElement(nameRef.value!),
});
</script>
<template>
  <div class="flex flex-row items-center">
    <IconInline
      ref="iconRef"
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: (node as any)!.icon, isInput: true },
          isEnabled: isInput,
          onApply: (newIcon) => getTx().update(node!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(node)"
      class="rounded text-center text-gray-700"
      :class="[...iconClass, isInput ? 'hover:cursor-pointer hover:bg-gray-100' : '']"
    />
    <!-- Name -->
    <NativeInput
      v-if="isInput"
      id="name"
      ref="nameRef"
      class="flex-shrink-0 transition-colors duration-150"
      :class="nameClass"
      :placeholder="nodeTypeName"
      is-input
      :value-type="NAME_TYPE"
      :variant="Variant.STEALTH"
      :model-value="(node as any).name"
      @update:model-value="(newValue) => getTx().update(node!, { name: newValue as string }, { debounce: 'long' })"
    />
    <span v-else :class="nameClass">
      {{ (node as any).name }}
    </span>
  </div>
</template>
