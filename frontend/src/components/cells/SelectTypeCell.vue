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
  (e: "deleteRight"): void;
  (e: "morphed"): void;
}>();

const context = useStatementContext();
const content: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

// handle content changes :ParseStatementInput
watch(content, (newContent) => {
  const endsInSpace = newContent.endsWith(" ") || newContent.endsWith(" "); // non-breaking spaces
  const contentTrim = newContent.trim();

  // if it matches an allowed keyword, apply the keyword
  if (endsInSpace && MODIFIER_BY_KEYWORD[contentTrim]) {
    context.setModifier(MODIFIER_BY_KEYWORD[contentTrim]);
    content.value = "";
    emit("morphed");
  } else if (endsInSpace && SYMBOL_TYPE_BY_KEYWORD[contentTrim]) {
    context.setSymbolType(SYMBOL_TYPE_BY_KEYWORD[contentTrim]);
    content.value = "";
    emit("morphed");
  } else if (endsInSpace && contentTrim == "enum") {
    context.setSymbolTypeEnum();
    content.value = "";
    emit("morphed");
  } else if (endsInSpace && (contentTrim == "#" || contentTrim == "//")) {
    context.morphToComment();
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
    @delete-right="emit('deleteRight')"
  />
</template>
