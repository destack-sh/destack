<script lang="ts" setup>
import { NAME_CONSTRAINT, toCamelName } from "@/language/const";
import { getNodeType } from "@/language/node";
import { ObjectType, PROPERTY_ENUM_BY_TYPE, ViewType, type AnyNodeData } from "@/proto/wire";
import { describeNode, isNode, type SomeNodeReferenceData } from "@/proto/wiring";
import type { Connection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/ui/icon";
import type { PopoverInfoIn } from "@/ui/popover";
import { getNativeConstraintProps, guardNativeNameInput } from "@/ui/view";
import { canvas } from "@/utils/globals";
import { computed, ref } from "vue";

const props = defineProps<{
  node: AnyNodeData | SomeNodeReferenceData;
  connection: Connection<"get", any>;
  isInput?: boolean;
}>();

const inputRef = ref<HTMLInputElement | null>(null);
const nodeType = computed(() => getNodeType(props.node));
const nodeTypeName = computed(() => (nodeType.value != null ? toCamelName(ObjectType, nodeType.value) : "???"));
const nodeProperties = computed(() => (nodeType.value != null ? PROPERTY_ENUM_BY_TYPE[nodeType.value] : null));
const hasName = computed(() => nodeProperties.value != null && "name" in nodeProperties.value);
const name = computed(() => (props.node as any).name);
const editingName = ref(false);
</script>
<template>
  <div :data-node-id="node.id" :data-node-ck="(node as any).ck" :data-node-type="node.metatype">
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
      class="w-5 rounded p-1 text-gray-700 hover:bg-gray-100 data-[popover=true]:bg-gray-100"
      :class="isInput ? 'cursor-pointer' : ''"
    />
    <!-- Name (editable & has name) -->
    <input
      v-if="hasName && editingName"
      ref="inputRef"
      v-outside.mousedown="{ callback: () => (editingName = false), delay: 100 }"
      class="ml-1 truncate rounded border-0 bg-gray-100 px-1 py-0.5 outline-none ring-0 focus:ring-0"
      spellcheck="false"
      :value="name ?? nodeTypeName"
      v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
      :size="(name?.length ?? nodeTypeName.length) + 3"
      @keydown.enter.stop.prevent="editingName = false"
      @input="
        guardNativeNameInput($event, (node as any).name, (newValue) => {
          if (!isNode(node)) throw new Error(`unexpected node: ${describeNode(node)}`);
          connection.tx.update(node!, { name: newValue }, { debounce: 'long' });
        })
      "
    />
    <button
      v-else
      :disabled="!isInput"
      class="ml-1 cursor-text truncate rounded px-1 py-0.5 hover:bg-gray-100"
      @click.stop.prevent="(editingName = true), $nextTick(() => inputRef?.focus())"
    >
      {{ (node as any).name ?? (node as any).title ?? "???" }}
    </button>
  </div>
</template>
