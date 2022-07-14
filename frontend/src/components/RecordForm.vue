<template>
  <form class="flex flex-col gap-3">
    <div class="sm:col-span-4" v-for="field in specs" :key="field.name">
      <label for="username" class="block text-sm font-medium text-gray-700">
        {{ field.name }}
      </label>
      <div class="mt-1 flex rounded-md shadow-sm">
        <FieldValueInterface
          :modelValue="props.modelValue[field.name]"
          :field="field"
          @update:modelValue="(value: any) => setRecordField(field, value)"
        />
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import { isFieldType, type FieldSpec, type FieldType, type RecordSpec } from "@/types";
import { computed, type Ref } from "vue";
import FieldValueInterface from "./FieldValueInterface.vue";

const props = defineProps<{
  spec: RecordSpec;
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

const specs: Ref<FieldSpec[]> = computed(() => {
  if (isFieldType(props.spec.type)) {
    return [
      {
        name: "",
        description: "",
        type: props.spec.type as FieldType,
      },
    ] as FieldSpec[];
  } else if (Array.isArray(props.spec.type)) {
    return props.spec.type as FieldSpec[];
  } else {
    return Object.values(props.spec.type) as FieldSpec[];
  }
});
</script>
