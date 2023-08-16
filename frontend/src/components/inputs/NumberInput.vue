<script lang="ts" setup>
import type { Field } from "@/gql/graphql";
import { ref, type Ref } from "vue";

const props = defineProps<{
  type: Field;
  modelValue?: number;
  preview?: boolean;
}>();

const inputRef: Ref<HTMLInputElement | null> = ref(null);

const emit = defineEmits<{
  (e: "update:modelValue", value: number): void;
}>();

defineExpose({
  focus: () => inputRef.value?.focus(),
  blur: () => inputRef.value?.blur(),
});
</script>
<template>
  <div v-if="preview" class="h-full w-full text-right">{{ modelValue }}&nbsp;</div>
  <input
    v-else
    ref="inputRef"
    class="w-full rounded-none border-none bg-transparent p-0 text-right outline-none ring-0 focus:ring-0"
    type="number"
    :value="modelValue"
    @input="emit('update:modelValue', Number.parseFloat(($event.target as any)?.value))"
    spellcheck="false"
  />
</template>
