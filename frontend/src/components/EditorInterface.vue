<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import RunInterface from "@/components/RunInterface.vue";
import { useEditorState, type Editor, type FileEditor, type RunEditor } from "@/utils/editor";
import { useScroll, watchDebounced } from "@vueuse/core";
import { onMounted, ref, watchEffect } from "vue";

const editorState = useEditorState();
const props = defineProps<{ editor: Editor }>();
const container = ref<HTMLElement | null>(null);

const { x, y, isScrolling } = useScroll(container);
const hasScrolledManually = ref(false);

// if user scrolls manually, do not restore scroll position
watchEffect(() => {
  if (isScrolling.value) {
    hasScrolledManually.value = true;
  }
});

// remember scroll position
watchDebounced(
  () => ({ x: x.value, y: y.value }),
  (scroll) => {
    editorState?.setEditorScroll(props.editor, scroll);
  },
  { debounce: 500, maxWait: 1000 }
);

// restore scroll position once after mount
onMounted(() => {
  // wait a bit to make sure the container is rendered
  // TODO @UX: get signal from child interfaces when they are ready
  setTimeout(() => {
    const scroll = props.editor.scroll;
    if (scroll && !hasScrolledManually.value) {
      x.value = scroll.x;
      y.value = scroll.y;
      console.log(`restored scroll in editor ${props.editor.id} to ${scroll.x}, ${scroll.y}`);
    }
  }, 1000);
});
</script>
<template>
  <div ref="container">
    <FileInterface v-if="editor.type == 'file'" :fileId="(editor as FileEditor).fileId" />
    <RunInterface v-else-if="editor.type == 'run'" :config="(editor as RunEditor).config" />
    <div v-else class="h-full w-full text-center">
      <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
    </div>
  </div>
</template>
