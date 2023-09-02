<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import { useStatementContext } from "@/state/statement";
import { computed, ref, watch, type Ref } from "vue";

defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void; (e: "toggleActions"): void }>();

const context = useStatementContext();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(context.statement.value.code ?? "");
context.syncText(
  text,
  computed(() => textRef.value?.focused)
);

function focus(position: "first" | "last" = "first") {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to blank where we want to keep editing smoothly)
  const focusEnd = context.statement.value.revision < 0 || position == "last";
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  textRef.value?.focus(focusEnd ? "last" : "first");
}

// morph back to blank if it's empty for smooth back and forth
watch(text, () => {
  if (text.value.trim() == "") {
    context.morphToBlank();
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
    @navigate-up="context.navigateUp"
    @navigate-down="context.navigateDown"
    @enter-start="context.insertAbove"
    @enter="context.insertBelow"
    @toggle-actions="emit('toggleActions')"
    @delete-start="context.deleteLeft"
    @delete-if-empty="context.deleteSelf"
    @paste="context.paste"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
  />
</template>
