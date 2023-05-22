<script lang="ts" setup>
import TiptapEditor from "@/components/TiptapEditor.vue";
import { useStatementContext } from "@/components/statement";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const context = useStatementContext();
const editorRef = ref<InstanceType<typeof TiptapEditor> | null>(null);
const content: Ref<string> = ref(context.statement.value.code ?? "");
context.syncText(
  content,
  computed(() => editorRef.value?.focused)
);

function focus() {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to comment where we want to keep editing smoothly)
  const focusEnd = context.statement.value.revision < 0;
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  editorRef.value?.focus(focusEnd);
  nextTick(() => editorRef.value?.focus(focusEnd));
}

// morph back to blank if it's empty for smooth back and forth
watch(content, () => {
  if (content.value.trim() == "<p></p>") {
    context.morphToBlank();
  }
});

defineExpose({
  focus,
  blur: () => editorRef.value?.blur(),
});
</script>
<template>
  <TiptapEditor
    ref="editorRef"
    :model-value="content || ''"
    @update:model-value="content = $event"
    @navigateUp="context.navigateUp"
    @navigateDown="context.navigateDown"
    @escape="context.escape"
    @enter="context.insertBelow"
    @delete-if-empty="context.deleteSelf"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class=""
  />
</template>
