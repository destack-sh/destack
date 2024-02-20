<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { Field } from "@/gql/graphql";
import { ref, type Ref } from "vue";

const props = defineProps<{
  type: Field;
  modelValue?: string;
  preview?: boolean;
  previewWidth?: number;
  previewHeight?: number;
  wrap?: boolean;
}>();
const inputRef: Ref<HTMLInputElement | null> = ref(null);

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

defineExpose({
  focus: () => inputRef.value?.focus(),
  blur: () => inputRef.value?.blur(),
});
</script>
<template>
  <AnnotatedText
    ref="inputRef"
    class="h-full w-full"
    :class="wrap ? 'whitespace-pre-wrap' : 'whitespace-nowrap'"
    :model-value="modelValue ?? ''"
    @update:modelValue="emit('update:modelValue', $event)"
    :readonly="preview"
  />
</template>
