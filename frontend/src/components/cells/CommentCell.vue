<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import { useEditorState } from "@/state/editor";
import DOMPurify from "dompurify";
import { marked } from "marked";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const context = useStatementContext();
const monacoEditorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);
const content: Ref<string> = ref(context.statement.value.code ?? "");
context.syncText(content);

const sanitizedHtml = computed(() => DOMPurify.sanitize(marked.parse(content.value || "")));

function focus() {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to comment where we want to keep editing smoothly)
  const focusEnd = context.statement.value.revision < 0;
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  monacoEditorRef.value?.focus(focusEnd);
  nextTick(() => monacoEditorRef.value?.focus(focusEnd));
}

// morph back to blank if it's empty for smooth back and forth
watch(content, () => {
  if (content.value.trim().length == 0) {
    context.morphToBlank();
  }
});

const editor = useEditorState();

defineExpose({
  focus,
  blur: () => monacoEditorRef.value?.blur(),
});
</script>
<template>
  <!-- Editing source markdown -->
  <MonacoEditor
    v-show="context.editing.value"
    ref="monacoEditorRef"
    :model-value="content || ''"
    @update:model-value="content = $event"
    @navigateUp="context.navigateUp"
    @navigateDown="context.navigateDown"
    @escape="context.escape"
    @enter="context.insertBelow"
    @delete-if-empty="context.deleteSelf"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    :line-number-offset="0"
    :line-number-shift-px="context.xOffset.value + 20"
    :style="{ marginLeft: -context.xOffset.value - 20 + 'px' }"
    hide-line-numbers
    language="markdown"
  />
  <!-- Show rendered markdown if not editing -->
  <div
    v-if="!context.editing.value"
    class="prose mt-[-1px]"
    :class="{
      'text-sm': editor.textSmall,
      'text-md': !editor.textSmall,
      'font-mono': editor.fontMono,
    }"
    v-html="sanitizedHtml"
  />
</template>
