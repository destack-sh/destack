<script lang="ts" setup>
import type { SimpleType } from "@/gql/graphql";
import { ref, type Ref } from "vue";

const props = defineProps<{
  type: SimpleType;
  modelValue?: string;
  preview?: boolean;
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
  <!-- :ForcedValueMinWidth -->
  <div v-if="preview" class="h-full w-full min-w-[150px] whitespace-pre-wrap">{{ modelValue }}&nbsp;</div>
  <textarea
    v-else
    ref="inputRef"
    class="h-fit w-full min-w-[300px] whitespace-pre rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
    type="text"
    :value="modelValue"
    @input="emit('update:modelValue', $event.target?.value)"
    spellcheck="false"
  />
</template>
