<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { MODIFIER_BY_KEYWORD, SYMBOL_TYPE_BY_KEYWORD } from "@/state/editor";
import { ref, watch, type Ref } from "vue";

defineProps<{ showDots?: boolean }>();

const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "morphed"): void;
}>();

const context = useStatementContext();
const content: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

const MAX_KEYWORD_LENGTH = [...Object.keys(MODIFIER_BY_KEYWORD), ...Object.keys(SYMBOL_TYPE_BY_KEYWORD)].reduce(
  (max, keyword) => Math.max(max, keyword.length),
  0
);

// parse content changes :ParseStatementInput
watch(content, (newContent) => {
  const endsInSpace = newContent.endsWith(" ") || newContent.endsWith(" "); // non-breaking spaces
  const includesNonalpha = !newContent.match(/^[a-zA-Z]*$/);
  const tooLong = newContent.length > MAX_KEYWORD_LENGTH;
  const newContentTrim = newContent.trim();

  // if it matches an allowed keyword, apply the keyword
  if (endsInSpace && MODIFIER_BY_KEYWORD[newContentTrim]) {
    context.setModifier(MODIFIER_BY_KEYWORD[newContentTrim]);
    content.value = "";
    emit("morphed");
  } else if (endsInSpace && SYMBOL_TYPE_BY_KEYWORD[newContentTrim]) {
    context.setSymbolType(SYMBOL_TYPE_BY_KEYWORD[newContentTrim]);
    content.value = "";
    emit("morphed");
  } else if (endsInSpace && newContentTrim == "enum") {
    context.setSymbolTypeEnum();
    content.value = "";
    emit("morphed");
  } else if (includesNonalpha || tooLong) {
    // auto-convert to comment if it can't be parsed anymore (keep content)
    newContent = newContent.replace(" ", " "); // replace non-breaking spaces
    context.morphToComment(newContent);
  }
});

defineExpose({
  focus: () => spanRef.value?.focus(),
  blur: () => spanRef.value?.blur(),
  content,
});
</script>
<template>
  <EditableSpan
    ref="spanRef"
    v-model="content"
    :readonly="context.readonly.value"
    @navigate-up="emit('navigateUp')"
    @navigate-down="emit('navigateDown')"
    @navigate-left="emit('navigateLeft')"
    @navigate-right="emit('navigateRight')"
    @enter="emit('enter')"
    @escape="emit('escape')"
    @delete-left="emit('deleteLeft')"
  />
</template>
