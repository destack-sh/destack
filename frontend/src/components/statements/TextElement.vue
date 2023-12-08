<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { StatementType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { computed, ref, type Ref } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(props.statement.text ?? "");
const textSync = syncProperty({
  read: () => (text.value = props.statement.text ?? ""),
  write: () => ops.symbol.updateStatementText(null, props.statement.id, props.statement.text ?? "", text.value),
  debounceMs: 500,
  debounceMaxWait: 3000,
});
const headingLevel = computed(() => props.statement.headingLevel ?? 0);

function focus(position: "first" | "last" = "first") {
  textRef.value?.focus("last"); // we always want to focus the end of the text
}

function blur() {
  textRef.value?.blur();
}

function onInput() {
  textSync.onLocalWrite();
  // if text statement: morph to blank if empty
  // (only if we are the ones who caused the change)
  if (
    props.focused &&
    !props.readonly &&
    props.statement.type == StatementType.Text &&
    headingLevel.value == 0 &&
    text.value == ""
  ) {
    // should ideally be done in one tx, but we don't have that yet
    textSync.flushNow();
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: StatementType.Blank,
      headingLevel: null,
    });
  }
}

function onDeleteLeft() {
  if (headingLevel.value > 0) {
    // remove heading level
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: (text.value ?? "").length > 0 ? StatementType.Text : StatementType.Blank,
      headingLevel: null,
    });
  } else {
    // actually delete
    emit("deleteLeft");
  }
}

defineExpose({
  focus,
  blur,
  syncNow: () => textSync.flushNow(),
});
</script>
<template>
  <div
    class="relative w-full text-gray-900"
    :class="{
      'mt-2 text-2xl': headingLevel == 1,
      'mt-1 text-xl': headingLevel == 2,
      'mt-0.5 text-lg': headingLevel == 3,
      'font-semibold': headingLevel > 0,
    }"
    @click="textRef?.focusIfUnfocused"
  >
    <!-- Actual text -->
    <AnnotatedText
      ref="textRef"
      :model-value="text || ''"
      @update:model-value="(text = $event), onInput()"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter="emit('enter')"
      @enter-right="emit('enterRight')"
      @delete-left="onDeleteLeft"
      @delete-if-empty="emit('deleteSelf')"
      @paste="emit('paste')"
      :focused="focused"
      :readonly="readonly"
      :statement="statement"
      :minimal-mentions="headingLevel > 0"
    />
    <!-- Placeholder if empty -->
    <template v-if="text.length == 0">&nbsp;</template>
    <button
      v-if="text.length == 0"
      class="absolute bottom-0 left-0 -m-0.5 -mx-0.5 flex flex-row items-center rounded-sm p-0.5 transition-colors duration-75 hover:bg-orange-100"
      :class="[focused ? 'text-gray-400' : 'text-gray-300']"
      @click="textRef?.focus"
    >
      Text...
    </button>
  </div>
</template>
