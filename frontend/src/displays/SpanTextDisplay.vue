<template>
  <template v-for="span in displaySpans" :key="span.start">
    <mark
      v-if="span.markStyle != null"
      :class="span.markStyle"
      class="rounded-sm bg-yellow-300 px-1 py-0.5"
    >
      {{ span.text }}
      <span v-if="(span as LabeledSpan).label" class="pr-0.5 text-xs leading-tight text-orange-700">
        {{ (span as LabeledSpan).label }}
      </span>
    </mark>
    <span v-else class="font-normal">
      {{ span.text }}
    </span>
  </template>
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
  markStyle?: string;
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
  function makeSpan(
    start: number,
    end: number,
    label?: string,
    score?: number,
    markStyle?: string
  ) {
    return { start, end, text: text(start, end), label, score, markStyle } as DisplaySpan;
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
      markStyle: "",
    });

    lastPos = entitySpan.end;
  }
  displaySpans.push(makeSpan(lastPos, props.modelValue.text.length));

  return displaySpans;
});
</script>
