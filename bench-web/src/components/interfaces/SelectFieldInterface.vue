<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import type { Field } from "@/gql/graphql";

const props = defineProps<{
  modelValue?: Field;
  options: Field[];
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: Field): void;
}>();
</script>
<template>
  <ul class="flex flex-col">
    <li
      v-for="field in options"
      :key="field.ck"
      class="flex cursor-pointer flex-row items-center whitespace-nowrap px-1 py-[3px] text-gray-900 hover:bg-amber-100"
      @click="emit('update:modelValue', field)"
    >
      <TypePreview :type="field" />
      <span class="ml-1.5">{{ field.name }}</span>
    </li>
  </ul>
</template>
