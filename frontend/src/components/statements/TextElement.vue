<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { StatementType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<StatementProps>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(props.statement.text ?? "");
syncProperty({
  value: text,
  editing: computed(() => textRef.value?.focused),
  read: () => (text.value = props.statement.text ?? ""),
  write: () => ops.symbol.updateStatementText(null, props.statement.id, props.statement.text ?? "", text.value),
  debounceMs: 500,
  debounceMaxWait: 3000,
});

function focus(position: "first" | "last" = "first") {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to blank where we want to keep editing smoothly)
  const focusEnd = props.statement.revision < 0 || position == "last";
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  textRef.value?.focus(focusEnd ? "last" : "first");
}

// morph back to blank if it's empty for smooth back and forth between text and blank
watch(text, () => {
  if (text.value.trim() == "") {
    ops.statement.morph(null, props.statement.id, props.statement.value, {
      type: StatementType.Blank,
      headingLevel: null,
    });
  }
});

defineExpose({
  focus,
  blur: () => textRef.value?.blur(),
});
</script>
<template>
  <AnnotatedText
    ref="textRef"
    :model-value="text || ''"
    @update:model-value="text = $event"
    @navigate-up="emit('navigateUp')"
    @navigate-down="emit('navigateDown')"
    @enter-start="emit('enterLeft')"
    @enter="emit('enter')"
    @toggle-actions="emit('openActions')"
    @delete-start="emit('deleteLeft')"
    @delete-if-empty="emit('deleteSelf')"
    @paste="emit('paste')"
    :focused="focused"
    :readonly="readonly"
  />
</template>
