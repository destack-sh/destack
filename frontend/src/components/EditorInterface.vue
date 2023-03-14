<script lang="ts" setup>
import FileInterface from "@/components/FileInterface.vue";
import RunInterface from "@/components/RunInterface.vue";
import RunsInterface from "@/components/RunsInterface.vue";
import {
  EDITOR_INTERFACE_STATE,
  useEditorState,
  type Editor,
  type EditorInterfaceState,
  type FileEditor,
  type RunEditor,
} from "@/state/editor";
import { useScroll, watchDebounced } from "@vueuse/core";
import { computed, onMounted, provide, ref, watchEffect } from "vue";

const editorState = useEditorState();
const props = defineProps<{ editor: Editor }>();
const containerRef = ref<InstanceType<typeof FileInterface> | null>(null);
const focused = computed(() => editorState.focusedEditorId == props.editor.id);

const { x, y, isScrolling } = useScroll(computed(() => containerRef.value?.$el));
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
  { debounce: 200, maxWait: 1000 }
);
// restore scroll position once after mount
function restoreScroll() {
  try {
    const scroll = props.editor.scroll;
    if (scroll && !hasScrolledManually.value) {
      x.value = scroll.x;
      y.value = scroll.y;
      console.log(`restored scroll in editor ${props.editor.id} to ${scroll.x}, ${scroll.y}`);
    }
  } catch (e) {
    // TODO @Robustness: fix scroll restoration
    console.warn("unable to restore scroll", e);
  }
}
onMounted(() => {
  // wait a bit to make sure the container is rendered
  // TODO @UX: fire scroll restore on some readiness signal (from child interfaces?)
  setTimeout(restoreScroll, 1000);
});

// generic editor interface state
const editorInterfaceState: EditorInterfaceState = {
  get<T>(key: string, default_?: T): T {
    if (props.editor.localState[key] === undefined && default_ !== undefined) {
      this.set(key, default_);
    }
    return props.editor.localState[key] as T;
  },
  set(key: string, state: unknown) {
    editorState.setEditorState(props.editor, key, state);
  },
};
provide(EDITOR_INTERFACE_STATE, editorInterfaceState);
</script>
<template>
  <FileInterface
    ref="containerRef"
    v-if="editor.type == 'file'"
    :editorId="editor.id"
    :fileId="(editor as FileEditor).fileId"
    :focused="focused"
    :state="editor.localState"
    @update:state="Object.assign(editor.localState, $event)"
    @close="editorState.closeEditor(editor)"
  />
  <RunInterface
    ref="containerRef"
    v-else-if="editor.type == 'run'"
    :editorId="editor.id"
    :focused="focused"
    :runnableId="(editor as RunEditor).symbolId"
    :runnableType="(editor as RunEditor).symbolType"
    @close="editorState.closeEditor(editor)"
  />
  <RunsInterface v-else-if="editor.type == 'runs'" :focused="focused" @close="editorState.closeEditor(editor)" />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-500">cannot render editor of type {{ editor.type }}</span>
  </div>
</template>
