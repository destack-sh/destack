<script lang="ts" setup>
import FileEditorInterface from "@/components/editors/FileEditor.vue";
import StatementEditorInterface from "@/components/editors/StatementEditor.vue";
import LaunchEditorInterface from "@/components/editors/LaunchEditor.vue";
import { useActiveScroll } from "@/composables/useScroll";
import {
  provideEditorContext,
  useBenchState,
  type EditorContext,
  type Editor,
  type FileEditor,
  LaunchEditor,
  StatementEditor,
} from "@/state/bench";
import { useEventListener } from "@vueuse/core";
import { computed, onBeforeUnmount, onMounted, ref, toRef } from "vue";

const bench = useBenchState();
const props = defineProps<{ editor: Editor; containerEl: HTMLElement | null }>();
const containerRef = ref<InstanceType<typeof FileEditorInterface> | null>(null);
const containerEl = toRef(props, "containerEl");
const focused = computed(() => bench.focusedEditorId == props.editor.id);

const scroll = useActiveScroll(containerEl);
// auto focus on click
useEventListener(containerEl, "click", () => {
  bench.focusEditor(props.editor);
});

const context = provideEditorContext(toRef(props, "editor"), containerRef, toRef(props, "containerEl"), scroll);

onMounted(() => {
  props.editor.onMounted?.(context);
});
onBeforeUnmount(() => {
  props.editor.onUnmounted?.(context);
});

defineExpose({
  context,
});
</script>
<template>
  <FileEditorInterface
    ref="containerRef"
    v-if="editor.type == 'file'"
    :editor="(context as EditorContext<FileEditor>)"
    :fileId="(editor as FileEditor).fileId"
    :focused="focused"
    @close="bench.closeEditor(editor)"
  />
  <StatementEditorInterface
    ref="containerRef"
    v-else-if="editor.type == 'statement'"
    :editor="(context as EditorContext<StatementEditor>)"
    :focused="focused"
    @close="bench.closeEditor(editor)"
  />
  <LaunchEditorInterface
    ref="containerRef"
    v-else-if="editor.type == 'launch'"
    :editor="(context as EditorContext<LaunchEditor>)"
    :focused="focused"
    @close="bench.closeEditor(editor)"
  />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
  </div>
</template>
