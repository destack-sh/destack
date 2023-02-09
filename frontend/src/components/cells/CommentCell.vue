<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import DOMPurify from "dompurify";
import { marked } from "marked";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();
const monacoEditorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);
const content: Ref<string> = ref(context.statement.value.code ?? "");
context.syncCode(content);

const sanitizedHtml = computed(() => DOMPurify.sanitize(marked.parse(content.value || "")));

defineExpose({
  focus: () => monacoEditorRef.value?.focus(),
  blur: () => monacoEditorRef.value?.blur(),
});
</script>
<template>
  <!-- TODO @Robustness: fix brief flicker before comment cell monaco is loaded -->
  <!-- unfortunately straightforward v-show instead of v-is seems to break focus -->
  <!-- Editing markdown -->
  <MonacoEditor
    v-if="context.editing.value"
    ref="monacoEditorRef"
    :model-value="content || ''"
    @update:model-value="content = $event"
    @navigateUp="context.navigateUp"
    @navigateDown="context.navigateDown"
    @escape="context.escape"
    @delete-if-empty="context.deleteSelf"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    :line-number-offset="0"
    :line-number-shift-px="context.xOffset.value + 20"
    :style="{ marginLeft: -context.xOffset.value - 20 + 'px' }"
    hide-line-numbers
    language="markdown"
  />
  <!-- Show rendered markdown -->
  <div v-else class="prose mt-[-1px] font-mono text-sm" v-html="sanitizedHtml" />
</template>
