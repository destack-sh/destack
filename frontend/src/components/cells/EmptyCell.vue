<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { MODIFIER_BY_KEYWORD, SYMBOL_TYPE_BY_KEYWORD } from "@/state/editor";
import { ref, watch, type Ref } from "vue";

defineProps<{ showDots?: boolean }>();

const context = useStatementContext();
const content: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

// handle content changes
watch(content, (newContent) => {
  const endsInSpace = newContent.endsWith(" ") || newContent.endsWith(" "); // non-breaking cell
  const contentTrim = newContent.trim();

  // if it matches an allowed keyword, apply the keyword
  if (endsInSpace && MODIFIER_BY_KEYWORD[contentTrim]) {
    context.setModifier(MODIFIER_BY_KEYWORD[contentTrim]);
  } else if (endsInSpace && SYMBOL_TYPE_BY_KEYWORD[contentTrim]) {
    context.setSymbolType(SYMBOL_TYPE_BY_KEYWORD[contentTrim]);
  } else if (endsInSpace && contentTrim == "enum") {
    context.setSymbolTypeEnum();
  } else if (endsInSpace && (contentTrim == "#" || contentTrim == "//")) {
    context.morphToComment();
  }
});

defineExpose({
  focus: () => spanRef.value?.focus(),
  defocus: () => spanRef.value?.defocus(),
});
</script>
<template>
  <EditableSpan
    ref="spanRef"
    v-model="content"
    :readonly="context.readonly.value"
    @enter="context.insertBelow"
    @delete-left="context.deleteSelf"
    @navigate-up="context.navigateUp"
    @navigate-down="context.navigateDown"
    @escape="context.escape"
  />
  <div
    v-if="showDots && content.length == 0"
    class="absolute bottom-0 mx-1 h-full w-full select-none text-gray-300 group-hover:opacity-100"
    :class="{ 'opacity-100': context.focused.value, 'opacity-0': !context.focused.value }"
  >
    ...
  </div>
</template>
