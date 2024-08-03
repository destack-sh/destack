<script lang="ts" setup>
import { ObjectType, PROPERTY_ENUM_BY_TYPE, ViewType, type AnyNodeData } from "@/proto/wire";
import type { Connection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { getNodeType, NAME_CONSTRAINT, toCamelName } from "@/language/utils";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import type { PopoverInfoIn } from "@/ui/popover";
import { computed } from "vue";
import { describeNode, isNode, type SomeNodeReferenceData } from "@/proto/wiring";

const props = defineProps<{
  node: AnyNodeData | SomeNodeReferenceData;
  connection: Connection<"get", any>;
  isDisabled?: boolean;
}>();

const nodeType = computed(() => getNodeType(props.node));
const nodeProperties = computed(() => (nodeType.value != null ? PROPERTY_ENUM_BY_TYPE[nodeType.value] : null));
</script>
<template>
  <div>
    <!-- NOTE :UX: hover preview for node references (files, pages, databases, ...) :NodePreviews -->
    <!-- Icon -->
    <IconInline
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.ICON,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: getNodeIcon(node!), isInput: true },
          isEnabled: nodeProperties != null && 'icon' in nodeProperties,
          onApply: (newIcon) => {
            if (!isNode(node)) throw new Error(`unexpected node: ${describeNode(node)}`);
            connection.tx.update(node, { icon: newIcon });
          },
        })
      "
      v-bind="getNodeIcon(node)"
      class="w-5 rounded p-1 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
    />
    <!-- Name (editable & has name) -->
    <input
      v-if="nodeProperties != null && 'name' in nodeProperties && !isDisabled"
      class="ml-1 truncate rounded border-0 px-1 py-0.5 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
      spellcheck="false"
      :value="'name' in node ? node.name : toCamelName(ObjectType, node.metatype)"
      :disabled="!('name' in node)"
      v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
      @input="
        guardNativeInput(NAME_CONSTRAINT, $event, (node as any).name, (newValue) => {
          if (!isNode(node)) throw new Error(`unexpected node: ${describeNode(node)}`);
          connection.tx.update(node!, { name: newValue }, { debounce: 'long' });
        })
      "
    />
    <span v-else class="ml-1 truncate rounded px-1 py-0.5 hover:bg-gray-100">
      {{ (node as any).name ?? (node as any).title ?? "???" }}
    </span>
  </div>
</template>
