<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { NAME_TYPE, TITLE_TYPE } from "@/language/core/type";
import { Transaction } from "@/language/runtime/transaction";
import { AnyNodeData, NodeType, Orientation, PROPERTY_ENUM_BY_TYPE } from "@/proto/wire";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { PopoverInfoIn } from "@/ui/popover";
import { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import { FocusAnchor, ViewEmits } from "@/views/common";
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
  hideIcon?: boolean;
  orientation?: Orientation;
  maxWidth?: number;
}>();
const emit = defineEmits<ViewEmits>();

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
  props.size == "regular" ? "w-5 mr-1" : "",
  props.size == "large" ? "w-5 text-base mr-1.5" : "",
  props.size == "title" ? "w-8 text-2xl mr-1.5" : "",
]);
const identifierClass = computed(() => [
  props.size == "regular" ? [props.isLight ? "" : "font-medium"] : "",
  props.size == "large" ? ["text-xl", props.isLight ? "" : "font-medium"] : "",
  props.size == "title" ? ["text-4xl", props.isLight ? "font-medium" : "font-bold"] : "",
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
  focusIcon: () => focusInElement(iconRef.value as MaybeElement),
  focusIdentifier: (anchor?: FocusAnchor) => identifierRef.value?.focus?.(anchor),
});
</script>
<template>
  <div
    class=""
    :class="orientation == Orientation.VERTICAL ? 'flex flex-col gap-y-0.5' : 'flex flex-row items-center'"
    :data-node-id="node.id"
    :data-node-ck="(node as any).ck"
    :data-node-type="node.metatype"
    aria-role="button"
    data-contextmenu-items="space.navigate.open"
  >
    <IconInline
      v-if="!hideIcon"
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
      role="button"
      aria-hidden
    />
    <!-- Identifier -->
    <NativeInput
      v-if="isInput"
      id="identifier"
      ref="identifierRef"
      class="flex-shrink-0 rounded text-gray-900"
      :style="{ maxWidth: `${identifierWidthMax}px` }"
      :class="identifierClass"
      :placeholder="nodeTypeName"
      is-input
      :value-type="identifierKind == 'name' ? NAME_TYPE : TITLE_TYPE"
      is-minimal
      :model-value="identifier"
      aria-hidden
      @update:model-value="
        (newValue) => getTx().update(node!, { [identifierKind!]: newValue as string }, { debounce: 'long' })
      "
      @navigate="emit('navigate', $event)"
    />
    <span v-else :class="identifierClass" class="truncate" :style="{ maxWidth: `${identifierWidthMax}px` }">
      {{ identifier }}
    </span>
    <!-- Metadata -->
    <NodeMetadata v-if="!isMinimal" :size="size" :node="node" :is-light="isLight" :class="metadataClass" />
  </div>
</template>
