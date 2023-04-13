<script lang="ts" setup>
import { useFocus } from "@vueuse/core";
import { ref } from "vue";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  suppressAllShortcuts?: boolean;
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
}>();

function deleteLeftIfEmpty(e: KeyboardEvent) {
  if (props.modelValue.length == 0) {
    emit("deleteLeft");
    e.stopPropagation();
    e.preventDefault();
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

const spanRef = ref<HTMLElement | null>(null);

// On focus we auto-select the end of the span always. Later (soon?),
// we'll want to set either focus start, end or all.

function focus() {
  spanRef.value?.focus();
  const selection = window.getSelection();
  if (selection && spanRef.value != null) {
    // select end of text
    if (spanRef.value.childNodes.length > 0) {
      selection.selectAllChildren(spanRef.value.childNodes[0]);
      selection.setBaseAndExtent(
        spanRef.value.childNodes[0],
        props.modelValue.length,
        spanRef.value.childNodes[0],
        props.modelValue.length
      );
    } else {
      // span has no text content yet
      selection.selectAllChildren(spanRef.value);
      selection.collapseToEnd();
    }
  }
}

function blur() {
  // lose selection
  spanRef.value?.blur();
  const selection = window.getSelection();
  if (selection) {
    selection.removeAllRanges();
  }
}

const { focused } = useFocus(spanRef);

defineExpose({
  focus,
  blur,
  focused,
  modelValue: props.modelValue,
});
</script>
<template>
  <!-- mousetrap class to enable keyboard shortcuts while editing -->
  <!-- except (undo redo which we want to keep native) -->
  <!-- https://craig.is/killing/mice#api.trigger -->
  <span
    tabindex="-1"
    spellcheck="false"
    ref="spanRef"
    class="outline-none"
    :class="suppressAllShortcuts ? '' : 'mousetrap mousetrap-no-do'"
    :contenteditable="!readonly"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.exact.left="navigateLeftIfAtStart"
    @keydown.exact.right="navigateRightIfAtEnd"
    @keydown.enter.exact.prevent="emit('enter', modelValue)"
    @keydown.backspace.exact="deleteLeftIfEmpty"
    @keydown.escape.prevent="emit('escape')"
    @input="emit('update:modelValue', spanRef?.innerText.replace('\n', '') ?? '')"
  >
    {{ modelValue }}
  </span>
</template>
