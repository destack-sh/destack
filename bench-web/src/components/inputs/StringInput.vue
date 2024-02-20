<script lang="ts" setup>
import type { Field } from "@/gql/graphql";
import { onMounted, ref, watch, type Ref } from "vue";

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

function autosize() {
  const el = inputRef.value;
  if (el == null) return;
  el.style.height = "auto";
  el.style.height = el.scrollHeight + "px";
}

onMounted(() => autosize());

defineExpose({
  focus: () => inputRef.value?.focus(),
  blur: () => inputRef.value?.blur(),
});
</script>
<template>
  <div v-if="preview" class="h-full w-full" :class="wrap ? 'whitespace-pre-wrap' : 'whitespace-nowrap'">
    {{ modelValue }}&nbsp;
  </div>
  <textarea
    v-else
    ref="inputRef"
    class="w-full min-w-[300px] max-w-full whitespace-pre-wrap break-words rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    :style="{ minHeight: previewHeight + 'px' }"
    wrap="hard"
    type="text"
    :value="modelValue"
    @input="
      emit('update:modelValue', ($event.target as any)?.value);
      autosize();
    "
    spellcheck="false"
  />
</template>
