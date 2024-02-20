<script lang="ts" setup>
import CodeBlock from "@/components/basic/CodeBlock.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import { TypeTag, type Field, TypeHint } from "@/gql/graphql";
import { onMounted, ref, watch, type Ref, nextTick } from "vue";

const props = defineProps<{
  type: Field;
  modelValue?: string;
  preview?: boolean;
  previewWidth?: number;
  previewHeight?: number;
  wrap?: boolean;
}>();
const inputRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "close"): void;
  (e: "enter"): void;
}>();

defineExpose({
  focus: () => {
    inputRef.value?.focus();
    nextTick(() => inputRef.value?.focus("last"));
  },
  blur: () => inputRef.value?.blur(),
});
</script>
<template>
  <!-- TODO @Robustness: auto-detect highlight language (but make it consistent across rows?) -->
  <CodeBlock
    v-if="preview"
    :model-value="modelValue ?? ''"
    :language="type.hint == TypeHint.Html ? 'html' : 'python'"
  />
  <MonacoEditor
    v-else
    ref="inputRef"
    class="mousetrap-ignore w-full min-w-[300px] max-w-full border-0 p-0"
    :style="{ minHeight: previewHeight + 'px' }"
    type="text"
    :model-value="modelValue ?? ''"
    language="python"
    hide-line-numbers
    enter-is-execute
    focused
    @execute="emit('enter')"
    @update:model-value="emit('update:modelValue', $event)"
    @escape="emit('close')"
    spellcheck="false"
  />
</template>
