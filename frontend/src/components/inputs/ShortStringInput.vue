<script lang="ts" setup>
import { TypeHint, type Field } from "@/gql/graphql";
import { ref, type Ref } from "vue";

const props = defineProps<{
  type: Field;
  modelValue?: string;
  preview?: boolean;
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
  <a
    v-if="preview && type.hint == TypeHint.Url"
    class="h-full w-full text-gray-500 underline decoration-gray-300 underline-offset-4"
    :class="wrap ? 'whitespace-pre-wrap' : 'whitespace-nowrap'"
    :href="modelValue"
    target="_blank"
  >
    <span @click.stop>{{ modelValue }}</span>
    <template v-if="(modelValue ?? '') == ''">&nbsp;</template></a
  >
  <div v-else-if="preview" class="h-full w-full" :class="wrap ? 'whitespace-pre-wrap' : 'whitespace-nowrap'">
    {{ modelValue }}&nbsp;
  </div>
  <input
    v-else
    ref="inputRef"
    class="min-w-[300px] max-w-full rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    :class="wrap ? 'whitespace-pre-wrap' : 'whitespace-nowrap'"
    type="text"
    :value="modelValue"
    @input="emit('update:modelValue', $event.target?.value)"
    spellcheck="false"
  />
</template>
