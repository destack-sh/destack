<script lang="ts" setup>
import { ViewData, NodeType, Variant } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: any } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <div
      class="w-fit rounded border border-gray-200"
      :class="[variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1' : '']"
    >
      nocheckin: Value {{ props.valueType?.benchType ?? props.valueType?.primitiveType }}
    </div>
  </ViewContentWrapper>
</template>
