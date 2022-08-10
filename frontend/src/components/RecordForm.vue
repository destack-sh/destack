<template>
  <form class="flex flex-col gap-3">
    <div class="sm:col-span-4" v-for="field in specs" :key="field.name">
      <label for="username" class="block text-sm font-medium text-gray-700">
        {{ field.name }}
        <span v-if="isOptional(field)" class="font-normal text-gray-500">(optional)</span>
      </label>
      <div class="mt-1 flex rounded-md">
        <FieldValueInterface
          :modelValue="props.modelValue[field.name]"
          :field="field"
          @update:modelValue="(value: any) => setRecordField(field, value)"
          :required="!isOptional(field)"
        />
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import { isFieldType, unravelFieldSpec, type FieldSpec, type FieldTypePrimitive, type RecordSpec } from "@/types";
import { computed, type Ref } from "vue";
import FieldValueInterface from "./FieldValueInterface.vue";

const props = defineProps<{
  spec: RecordSpec[];
  modelValue: Record<string, any>;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>): void;
}>();

function setRecordField(field: FieldSpec, value: any) {
  const newRecord: Record<string, any> = { ...props.modelValue };
  newRecord[field.name] = value;
  emit("update:modelValue", newRecord);
}

function isOptional(field: FieldSpec): boolean | undefined {
  return isFieldType(field.type) && (field.type as FieldTypePrimitive).optional;
}

const specs: Ref<FieldSpec[]> = computed(() => unravelFieldSpec(props.spec));
</script>
