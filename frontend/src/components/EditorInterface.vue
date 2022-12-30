<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import {
  EDITOR_INTERFACE_STATE,
  useEditorState,
  type Editor,
  type EditorInterfaceState,
  type FileEditor,
} from "@/state/editor";
import { useScroll, watchDebounced } from "@vueuse/core";
import { onMounted, provide, ref, watchEffect } from "vue";

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

// generic editor interface state
const editorInterfaceState: EditorInterfaceState = {
  get(key: string, default_?: unknown) {
    if (props.editor.localState[key] === undefined && default_ !== undefined) {
      this.set(key, default_);
    }
    return props.editor.localState[key];
  },
  set(key: string, state: unknown) {
    editorState.setEditorState(props.editor, key, state);
  },
};
provide(EDITOR_INTERFACE_STATE, editorInterfaceState);
</script>
<template>
  <div ref="container">
    <FileInterface
      v-if="editor.type == 'file'"
      :fileId="(editor as FileEditor).fileId"
      :state="editor.localState"
      @update:state="Object.assign(editor.localState, $event)"
    />
    <!-- <RunInterface v-else-if="editor.type == 'execute'" :config="(editor as RunEditor).config" /> -->
    <div v-else class="h-full w-full text-center">
      <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
    </div>
  </div>
</template>
