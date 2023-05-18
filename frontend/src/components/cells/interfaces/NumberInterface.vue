<script lang="ts" setup>
import type { SimpleType } from "@/gql/graphql";

const props = defineProps<{
  type: SimpleType;
  modelValue?: number;
  preview?: number;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: number): void;
}>();
</script>
<template>
  <span v-if="preview">{{ modelValue }}</span>
  <input
    v-else
    class="rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    type="number"
    :value="modelValue"
    @input="emit('update:modelValue', Number.parseFloat($event.target?.value))"
    spellcheck="false"
  />
</template>
