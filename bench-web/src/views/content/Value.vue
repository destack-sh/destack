<script lang="ts" setup>
import { ViewData, NodeType, Variant } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef } from "vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { getViewForValueType, packValue, unpackValue } from "@/system/value";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: any } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const valueView = computed(() => {
  if (props.valueType == null) return { viewType: null, props: null };
  else return getViewForValueType(props.valueType);
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- nocheckin: UX: render Variable properly -->
    <component
      v-if="
        valueType != null && valueView != null && valueView.viewType != null && hasViewComponent(valueView.viewType)
      "
      :is="getViewComponent(valueView.viewType)"
      class="ml-auto flex-shrink-0"
      v-bind="valueView.props"
      :modelValue="unpackValue(modelValue, valueType)"
      @update:modelValue="$emit('update:modelValue', packValue($event, valueType!))"
    />
    <div v-else class="flex px-1 py-0.5 flex-row items-center text-warning-600">
      <i class="fas fa-empty-set" />
      <span class="ml-1.5">No View for Value Type</span>
    </div>
  </ViewContentWrapper>
</template>
