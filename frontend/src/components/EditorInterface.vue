<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { provideEditorContext, useBenchState, type EditorContext, type Editor, type FileEditor } from "@/state/editor";
import { computed, ref, toRef } from "vue";

const editorState = useBenchState();
const props = defineProps<{ editor: Editor; containerEl: HTMLElement | null }>();
const containerRef = ref<InstanceType<typeof FileInterface> | null>(null);
const focused = computed(() => editorState.focusedEditorId == props.editor.id);
useActiveScroll(toRef(props, "containerEl"));

const context = provideEditorContext(toRef(props, "editor"), toRef(props, "containerEl"));

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
    @close="editorState.closeEditor(editor)"
  />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
  </div>
</template>
