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

function navigateLeftIfAtStart(event: any) {
  // check if cursor is at the start of the text
  const selection = window.getSelection();
  if (selection && selection.anchorOffset == 0) {
    emit("navigateLeft");
    event.preventDefault();
    event.stopPropagation();
  }
}

function navigateRightIfAtEnd(event: any) {
  // check if cursor is at the end of the text
  const selection = window.getSelection();
  if (selection && selection.anchorOffset == props.modelValue.length) {
    emit("navigateRight");
    event.preventDefault();
    event.stopPropagation();
  }
}

const span = ref<HTMLElement | null>(null);

defineExpose({ focus: () => span.value?.focus(), defocus: () => span.value?.blur() });
</script>
<template>
  <!-- mousetrap class to enable keyboard shortcuts while editing -->
  <!-- https://craig.is/killing/mice#api.trigger -->
  <span
    tabindex="-1"
    spellcheck="false"
    ref="span"
    class="mousetrap outline-none"
    :contenteditable="!readonly"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.exact.left="navigateLeftIfAtStart"
    @keydown.exact.right="navigateRightIfAtEnd"
    @keydown.enter.exact.prevent="emit('enter', modelValue)"
    @keydown.backspace="deleteLeftIfEmpty"
    @keydown.delete="deleteRightIfAtEnd"
    @keydown.escape.prevent="emit('escape')"
    @input="emit('update:modelValue', span?.innerText ?? '')"
  >
    {{ modelValue }}
  </span>
</template>
