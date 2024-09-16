<script lang="ts" setup>
import { NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import type { PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getViewForValueType } from "@/ui/view";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    modelValue?: any;
    preparedConnection?: PreparedGetConnection;
  } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const valueView = computed(() => {
  if (props.valueType == null) return null;
  else return getViewForValueType(props.valueType);
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <component
      :is="getViewComponent(valueView.type)"
      v-if="valueType != null && valueView?.type != null && hasViewComponent(valueView.type)"
      class="ml-auto flex-shrink-0"
      v-bind="{ isInput: true, variant: props.variant, valueType, ...valueView }"
      :model-value="modelValue"
      :prepared-connection="props.preparedConnection"
      @update:model-value="$emit('update:modelValue', $event)"
      @apply="$emit('apply', $event)"
    />
    <div v-else class="flex flex-row items-center px-1 py-0.5 text-warning-600">
      <i class="fas fa-empty-set" />
      <span class="ml-1.5">No View for Type {{ valueType }}</span>
    </div>
  </ViewContentWrapper>
</template>
