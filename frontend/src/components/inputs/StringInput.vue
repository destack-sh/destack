<script lang="ts" setup>
import type { SimpleType } from "@/gql/graphql";
import { ref, type Ref } from "vue";

const props = defineProps<{
  type: SimpleType;
  modelValue?: string;
  preview?: boolean;
  previewWidth?: number;
  previewHeight?: number;
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
  <div v-if="preview" class="h-full w-full whitespace-pre-wrap">{{ modelValue }}&nbsp;</div>
  <textarea
    v-else
    ref="inputRef"
    class="emin-w-[300px] h-fit max-w-full whitespace-pre-wrap break-words rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    :style="{ minHeight: previewHeight + 'px' }"
    wrap="hard"
    type="text"
    :value="modelValue"
    @input="emit('update:modelValue', $event.target?.value)"
    spellcheck="false"
  />
</template>
