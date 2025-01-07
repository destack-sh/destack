<script lang="ts" setup>
import { ViewData, NodeType, PathData, TypeKind, BenchType, FieldType, StructType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import Picker from "@/views/content/Picker.vue";
import { makeTypeConstraint, makeTypeInfo } from "@/language/field";
import { ICON_BY_STRUCT_TYPE, makeIcon } from "@/ui/icon";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "valueType" | "isInput" | "isDisabled" | "isMinimal">
  >
>();
const modelValue = defineModel<PathData | undefined>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodeValueType = computed(() =>
  makeTypeInfo({
    kind: TypeKind.NODE,
    benchType: BenchType.FIELD,
    constraint: makeTypeConstraint({
      nodeScopePtr: props.valueType?.constraint?.nodeScopePtr,
      nodeSubtypes: [FieldType.INPUT, FieldType.OUTPUT, FieldType.VARIABLE],
    }),
  }),
);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- NOTE :Incomplete: Path view just forwards to Picker for now with some wrapping  :OverloadedPicker -->
    <!-- (obviously we want a richer Path view that supports all sorts of Paths) -->
    <Picker
      id="picker"
      :title="title ?? 'Path'"
      :icon="icon ?? ICON_BY_STRUCT_TYPE[StructType.PATH]"
      :value-type="nodeValueType"
      :is-input="isInput"
      :is-disabled="isDisabled"
      :is-minimal="isMinimal"
    />
  </div>
</template>
