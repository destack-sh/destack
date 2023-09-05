<script lang="ts" setup>
import { VALID_DESCRIPTION_REGEXP, VALID_NAME_REGEXP } from "@/utils/validation";
import { useFocus } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  suppressAllShortcuts?: boolean;
  regex?: string | RegExp | "name" | "description";
}>();

const regexp = computed(() => {
  if (props.regex == null) return null;
  if (props.regex == "name") {
    return VALID_NAME_REGEXP;
  } else if (props.regex == "description") {
    return VALID_DESCRIPTION_REGEXP;
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

function focus(pos: "first" | "last" = "first") {
  spanRef.value?.focus();
  if (pos == "first") {
    selectStart();
  } else if (pos == "last") {
    selectEnd();
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

function selectStart() {
  const selection = window.getSelection();
  if (selection && spanRef.value != null) {
    // select start of text
    if (spanRef.value.childNodes.length > 0) {
      selection.selectAllChildren(spanRef.value.childNodes[0]);
      selection.setBaseAndExtent(spanRef.value.childNodes[0], 0, spanRef.value.childNodes[0], 0);
    } else {
      // span has no text content yet
      selection.selectAllChildren(spanRef.value);
      selection.collapseToStart();
    }
  }
}

const { focused } = useFocus(spanRef);

function onInput(e: InputEvent) {
  const value = (e.target as HTMLElement).innerText;
  if (regexp.value != null && !regexp.value.test(value)) {
    const currentPos = window.getSelection()?.anchorOffset ?? 0;
    // invalid input, revert
    (e.target as HTMLElement).innerText = props.modelValue;
    // restore cursor position to where it was before
    const selection = window.getSelection();
    if (selection) {
      selection.collapse((e.target as HTMLElement).childNodes[0], currentPos - 1);
    }
  } else {
    emit("update:modelValue", fromNbsp(value));
  }
}

function toNbsp(s: string) {
  return s.replace(/ /g, "\u00a0");
}

function fromNbsp(s: string) {
  return s.replace(/\u00a0/g, " ");
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
    class="whitespace-pre-wrap outline-none"
    :class="suppressAllShortcuts ? '' : 'mousetrap mousetrap-no-do'"
    :contenteditable="!readonly"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.exact.left="navigateLeftIfAtStart"
    @keydown.exact.right="navigateRightIfAtEnd"
    @keydown.enter.exact.prevent="emit('enter', fromNbsp(modelValue))"
    @keydown.backspace.exact="deleteLeftIfEmpty"
    @keydown.escape.prevent="emit('escape')"
    @input="e => onInput(e as InputEvent)"
  >
    {{ modelValue }}
  </span>
</template>
