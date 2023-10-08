<script lang="ts" setup>
import CodeBlock from "@/components/basic/CodeBlock.vue";
import { TypeTag, type Field, TypeHint } from "@/gql/graphql";
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
  <!-- TODO @Robustness: auto-detect highlight language (but make it consistent across rows?) -->
  <!-- TODO @UX: provide proper monaco editor for CodeInput editing -->
  <CodeBlock
    v-if="preview"
    :model-value="modelValue ?? ''"
    :language="type.hint == TypeHint.Html ? 'html' : 'python'"
  />
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
