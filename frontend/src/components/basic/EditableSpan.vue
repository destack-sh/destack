<script lang="ts" setup>
import { VALID_NAME_REGEXP } from "@/utils/validation";
import { useFocus } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  suppressAllShortcuts?: boolean;
  regex?: string | RegExp | "name";
}>();

const regexp = computed(() => {
  if (props.regex == null) return null;
  if (props.regex == "name") {
    return VALID_NAME_REGEXP;
  }
  if (typeof props.regex == "string") {
    return new RegExp(props.regex);
  }
  return props.regex;
});

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
  selectEnd();
}

function blur() {
  // lose selection
  spanRef.value?.blur();
  const selection = window.getSelection();
  if (selection) {
    selection.removeAllRanges();
  }
}

function selectAll() {
  const selection = window.getSelection();
  if (selection && spanRef.value != null) {
    selection.selectAllChildren(spanRef.value);
  }
}

function selectEnd() {
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
const { focused } = useFocus(spanRef);

function onInput(e: InputEvent) {
  const value = (e.target as HTMLElement).innerText;
  if (regexp.value != null && !regexp.value.test(value)) {
    // invalid input, revert
    (e.target as HTMLElement).innerText = props.modelValue;
    // move cursor to end
    selectEnd();
  } else {
    emit("update:modelValue", value);
  }
}

defineExpose({
  focus,
  blur,
  selectAll,
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
    @input="e => onInput(e as InputEvent)"
  >
    {{ modelValue }}
  </span>
</template>
