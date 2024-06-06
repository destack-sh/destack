<script lang="ts" setup>
import { ObjectType, PROPERTY_ENUM_BY_TYPE, ViewType, type AnyNodeData } from "@/proto/wire";
import type { GraphConnection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import type { PopoverInfoIn } from "@/utils/menu";
import { computed } from "vue";

const props = defineProps<{
  node: AnyNodeData;
  connection: GraphConnection<"get", any>;
}>();

const nodeMetatype = computed(() => props.node?.metatype);
const nodeProperties = computed(() => (nodeMetatype.value != null ? PROPERTY_ENUM_BY_TYPE[nodeMetatype.value] : null));
</script>
<template>
  <div>
    <!-- Icon -->
    <IconInline
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.ICON,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: getNodeIcon(node!) },
          isEnabled: nodeProperties != null && 'icon' in nodeProperties,
          onApply: (newIcon) => connection.tx.update(node!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(node)"
      class="w-5 rounded p-1 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
    />
    <!-- Name -->
    <input
      class="ml-1 truncate rounded border-0 px-1 py-0.5 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
      spellcheck="false"
      :value="'name' in node ? node.name : toCamelName(ObjectType, node.metatype)"
      :disabled="!('name' in node)"
      @input="
        (event) => connection.tx.update(node!, { name: (event.target as HTMLInputElement).value }, { debounce: 'long' })
      "
    />
  </div>
</template>
