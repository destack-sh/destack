<template>
  <component
    :is="componentFor(field)"
    :modelValue="props.modelValue"
    @update:modelValue="(value: any) => $emit('update:modelValue', value)"
    v-bind="componentProps(field)"
    :name="field.name"
    :id="field.name"
  />
</template>
<script lang="ts" setup>
import BooleanInterface from "@/interfaces/BooleanInterface.vue";
import EnumInterface from "@/interfaces/EnumInterface.vue";
import MissingInterface from "@/interfaces/MissingInterface.vue";
import NumberInterface from "@/interfaces/NumberInterface.vue";
import TextInterface from "@/interfaces/TextInterface.vue";
import { isFieldType, type FieldSpec, type FieldType } from "@/types";

const props = defineProps<{
  modelValue: any;
  field: FieldSpec;
}>();
defineEmits<{
  (e: "update:modelValue", value: any): void;
}>();

function componentWithProps(field: FieldSpec) {
  if (isFieldType(field.type)) {
    const fieldType = field.type as FieldType;
    if (fieldType._type == "ValueType") {
      if (["int32", "int64", "float32", "float64"].includes(fieldType.dtype)) {
        return [
          NumberInterface,
          {
            default: Number.parseFloat(fieldType.default),
            dtype: fieldType.dtype,
          },
        ];
      } else if ("bool" == fieldType.dtype) {
        return [
          BooleanInterface,
          {
            default: fieldType.default || false,
          },
        ];
      } else {
        // fall back to string
        return [
          TextInterface,
          {
            default: fieldType.default,
            dtype: fieldType.dtype,
          },
        ];
      }
    } else if (fieldType._type == "EnumType") {
      return [
        EnumInterface,
        {
          values: fieldType.values,
          default: fieldType.values[0],
        },
      ];
    }
  }

  console.error(`unexpected field: ${field}`, field);
  return [MissingInterface, { description: null }];
}

function componentFor(field: FieldSpec) {
  return componentWithProps(field)[0];
}

function componentProps(field: FieldSpec) {
  return componentWithProps(field)[1];
}
</script>
