<script lang="ts" setup>
import { type Field, TypeHint } from "@/gql/graphql";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{
  type: Field;
  modelValue?: string;
  preview?: boolean;
}>();

const inputRef: Ref<HTMLInputElement | null> = ref(null);
const inputType = computed(() => {
  if (props.type.hint == TypeHint.Date) return "date";
  else if (props.type.hint == TypeHint.Time) return "time";
  else if (props.type.hint == TypeHint.Datetime) return "datetime-local";
  else return "text";
});

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

defineExpose({
  focus: () => inputRef.value?.focus(),
  blur: () => inputRef.value?.blur(),
});
</script>
<template>
  <div class="w-full rounded-none border-none bg-transparent p-0 text-left">
    <!-- TODO @UX: use proper date picker -->
    <input
      v-if="!preview || (modelValue ?? '') != ''"
      ref="inputRef"
      :type="inputType"
      :value="modelValue"
      :readonly="preview"
      @input="emit('update:modelValue', ($event.target as any)?.value)"
      spellcheck="false"
      class="m-0 rounded-none border-none bg-transparent p-0 text-sm text-gray-900 outline-none ring-0 focus:ring-0"
    />
  </div>
</template>
