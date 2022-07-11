<template>
  <form class="flex flex-col gap-3">
    <div class="sm:col-span-4" v-for="field in specs" :key="field.name">
      <label for="username" class="block text-sm font-medium text-gray-700">
        {{ field.name }}
      </label>
      <div class="mt-1 flex rounded-md shadow-sm">
        <component
          :is="componentFor(field)"
          :modelValue="record[field.name]"
          @update:modelValue="(value: any) => setRecord(field, value)"
          v-bind="componentProps(field)"
          :name="field.name"
          :id="field.name"
        />
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import EnumInterface from "@/interfaces/EnumInterface.vue";
import MissingInterface from "@/interfaces/MissingInterface.vue";
import NumberInterface from "@/interfaces/NumberInterface.vue";
import TextInterface from "@/interfaces/TextInterface.vue";
import { isFieldType, type FieldSpec, type FieldType, type RecordSpec } from "@/types";
import { computed, reactive, type Ref } from "vue";

const props = defineProps<{
  spec: RecordSpec;
}>();
const record: Record<string, any> = reactive({});

function setRecord(field: FieldSpec, value: any) {
  console.log(`set ${field.name} = ${value}`);
  record[field.name] = value;
}

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
