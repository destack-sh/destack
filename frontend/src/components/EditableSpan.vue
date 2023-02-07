<script lang="ts" setup>
import { ref } from "vue";

const props = defineProps<{
  modelValue: string;
  placeholder?: string;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: string): void;
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enter", value: string): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "deleteRight"): void;
}>();

function deleteLeftIfEmpty() {
  if (props.modelValue.length == 0) {
    emit("deleteLeft");
  }
}

function deleteRightIfAtEnd() {
  // check if cursor is at the end of the text
  const selection = window.getSelection();
  if (props.modelValue.length > 0 && selection && selection.anchorOffset == props.modelValue.length) {
    emit("deleteRight");
  }
}

function navigateLeftIfAtStart() {
  // check if cursor is at the start of the text
  const selection = window.getSelection();
  if (selection && selection.anchorOffset == 0) {
    emit("navigateLeft");
  }
}

function navigateRightIfAtEnd() {
  // check if cursor is at the end of the text
  const selection = window.getSelection();
  if (selection && selection.anchorOffset == props.modelValue.length) {
    emit("navigateRight");
  }
}

const span = ref<HTMLElement | null>(null);

defineExpose({ focus: () => span.value?.focus(), defocus: () => span.value?.blur() });
</script>
<template>
  <span
    tabindex="-1"
    spellcheck="false"
    ref="span"
    class="w-full outline-none"
    :contenteditable="!readonly"
    @keydown.up.prevent="emit('navigateUp')"
    @keydown.down.prevent="emit('navigateDown')"
    @keydown.left="navigateLeftIfAtStart"
    @keydown.right="navigateRightIfAtEnd"
    @keydown.enter.prevent="emit('enter', modelValue)"
    @keydown.backspace="deleteLeftIfEmpty"
    @keydown.delete="deleteRightIfAtEnd"
    @keydown.escape.prevent="emit('escape')"
    @input="emit('update:modelValue', span?.innerText ?? '')"
  >
    {{ modelValue }}
  </span>
</template>
