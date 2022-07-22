<template>
  <span v-for="span in displaySpans" :key="span.start" :class="span.style">
    {{ span.text }}
  </span>
</template>
<script lang="ts" setup>
import type { FieldSpec } from "@/types";
import { computed } from "vue";

type Span = {
  start: number;
  end: number;
  text: string;
};

type LabeledSpan = Span & {
  label: string;
  score?: number;
};

type DisplaySpan = Span & {
  style?: string;
};

const props = defineProps<{
  modelValue: {
    text: string;
    tokens?: Span[];
    entities?: LabeledSpan[];
  };
  spec: FieldSpec[];
}>();

const displaySpans = computed(() => {
  function makeSpan(start: number, end: number, label?: string, score?: number, style?: string) {
    return { start, end, text: text(start, end), label, score, style } as DisplaySpan;
  }

  function text(start: number, end: number): string {
    return props.modelValue.text.slice(start, end);
  }
  if (props.modelValue.entities == null) {
    return [makeSpan(0, props.modelValue.text.length)];
  }

  const displaySpans: DisplaySpan[] = [];
  var lastPos = 0;
  for (const entitySpan of props.modelValue.entities) {
    if (lastPos != entitySpan.start) {
      displaySpans.push(makeSpan(lastPos, entitySpan.start));
    }

    displaySpans.push({
      ...entitySpan,
      text: text(entitySpan.start, entitySpan.end),
      style: "text-red-500",
    });

    lastPos = entitySpan.end;
  }
  displaySpans.push(makeSpan(lastPos, props.modelValue.text.length));

  return displaySpans;
});
</script>
