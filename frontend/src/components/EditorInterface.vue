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
import { computed, provide, ref } from "vue";

const editorState = useEditorState();
const props = defineProps<{ editor: Editor }>();
const containerRef = ref<InstanceType<typeof FileInterface> | null>(null);
const focused = computed(() => editorState.focusedEditorId == props.editor.id);

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
