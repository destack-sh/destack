<script lang="ts" setup>
import { ViewData, NodeType, ComputedValueData, ObjectType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import Path from "@/views/structs/Path.vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "valueType" | "isInput" | "isDisabled" | "isMinimal">
  >
>();
const modelValue = defineModel<ComputedValueData | undefined>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- NOTE :Incomplete: ComputedValue Expressions & Code -->
    <Path
      id="path"
      title="Source Path"
      :value-type="valueType"
      :is-input="isInput"
      :is-disabled="isDisabled"
      :is-minimal="isMinimal"
      :model-value="modelValue?.sourcePath"
      @update:model-value="
        (value) => emit('update:modelValue', { ...modelValue, metatype: ObjectType.COMPUTED_VALUE, sourcePath: value })
      "
    />
  </div>
</template>
