<script lang="ts" setup>
import type { SimpleType } from "@/gql/graphql";

const props = defineProps<{
  type: SimpleType;
  modelValue?: string;
  preview?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();
</script>
<template>
  <span v-if="preview">{{ modelValue }}</span>
  <input
    v-else
    class="rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    type="text"
    :value="modelValue"
    @input="emit('update:modelValue', $event.target?.value)"
    spellcheck="false"
  />
</template>
