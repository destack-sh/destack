<template>
  <form class="flex flex-col gap-3">
    <div class="sm:col-span-4" v-for="field in specs" :key="field.name">
      <label for="username" class="block text-sm font-medium text-gray-700">
        {{ field.name }}
        <span v-if="isOptional(field)" class="font-normal text-gray-500">(optional)</span>
      </label>
      <div class="mt-1 flex rounded-md">
        <ArtifactSelect hide-if-empty />
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import {
  isArtifactType,
  unravelConfigSpec,
  type ArtifactType,
  type ArtifactVersion,
  type ConfigSpec,
  type FieldSpec,
} from "@/types";
import { computed, type Ref } from "vue";
import ArtifactSelect from "./ArtifactSelect.vue";

const props = defineProps<{
  spec: ConfigSpec;
  modelValue: Record<string, ArtifactVersion>;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, ArtifactVersion>): void;
}>();

function setRecordField(field: FieldSpec, value: any) {
  const newRecord: Record<string, any> = { ...props.modelValue };
  newRecord[field.name] = value;
  emit("update:modelValue", newRecord);
}

function isOptional(field: FieldSpec): boolean | undefined {
  return isArtifactType(field.type) && (field.type as ArtifactType).optional;
}

const specs: Ref<FieldSpec[]> = computed(
  () => unravelConfigSpec(props.spec).filter((spec) => isArtifactType(spec.type)) as FieldSpec[]
);
</script>
