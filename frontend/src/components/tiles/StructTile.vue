<script lang="ts" setup>
import type { Field } from "@/state/statement";
import StructInterface from "@/components/interfaces/StructInterface.vue";

const props = defineProps<{
  fields: Field[];
  modelValue: Record<string, any>;
  readonly?: boolean;
  fullInputs?: boolean;
  active?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>): void;
  (e: "deleteSelf"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();
</script>
<template>
  <StructInterface
    :fields="props.fields"
    :model-value="props.modelValue"
    :readonly="props.readonly ?? false"
    :active="props.active ?? false"
    :fullInputs="props.fullInputs ?? false"
    @update:modelValue="emit('update:modelValue', $event)"
    @deleteSelf="emit('deleteSelf')"
    @navigateUp="emit('navigateUp')"
    @navigateDown="emit('navigateDown')"
  />
</template>
