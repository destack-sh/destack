<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import RunInterface from "@/components/RunInterface.vue";
import { useActiveScroll } from "@/composables/useScroll";
import {
  provideEditorContext,
  useBenchState,
  type EditorContext,
  type Editor,
  type FileEditor,
  RunEditor,
} from "@/state/editor";
import { useEventListener } from "@vueuse/core";
import { computed, onBeforeUnmount, onMounted, ref, toRef } from "vue";

const bench = useBenchState();
const props = defineProps<{ editor: Editor; containerEl: HTMLElement | null }>();
const containerRef = ref<InstanceType<typeof FileInterface> | null>(null);
const containerEl = toRef(props, "containerEl");
const focused = computed(() => bench.focusedEditorId == props.editor.id);

useActiveScroll(containerEl);
// auto focus on click
useEventListener(containerEl, "click", () => {
  bench.focusEditor(props.editor);
});

const context = provideEditorContext(toRef(props, "editor"), containerRef, toRef(props, "containerEl"));

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
  <FileInterface
    ref="containerRef"
    v-if="editor.type == 'file'"
    :editor="(context as EditorContext<FileEditor>)"
    :fileId="(editor as FileEditor).fileId"
    :focused="focused"
    @close="bench.closeEditor(editor)"
  />
  <RunInterface
    ref="containerRef"
    v-else-if="editor.type == 'run'"
    :editor="(context as EditorContext<RunEditor>)"
    :focused="focused"
    @close="bench.closeEditor(editor)"
  />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
  </div>
</template>
