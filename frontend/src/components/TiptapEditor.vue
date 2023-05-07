<script lang="ts" setup>
import { useEditor, EditorContent, Extension } from "@tiptap/vue-3";
import StarterKit from "@tiptap/starter-kit";
import { defineExpose, defineProps, defineEmits, ref, watch, type Ref } from "vue";
import { useAppearance } from "@/state/appearance";

const props = defineProps<{
  modelValue: string;
  focused: boolean;
  readonly?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "deleteIfEmpty"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "escape"): void;
  (e: "enter"): void;
  (e: "execute"): void;
}>();

const focused: Ref<boolean> = ref(false);

// custom shortcuts
const shortcutsExtension = Extension.create({
  addKeyboardShortcuts() {
    return {
      "Shift-Enter": () => {
        emit("enter");
        return true;
      },
      "Ctrl-Enter": () => {
        emit("execute");
        return true;
      },
      Escape: () => {
        emit("escape");
        return true;
      },
    };
  },
});
function onKeyDown(event: KeyboardEvent) {
  // should probably move all these into the shortcuts extension?
  if (event.key === "Backspace" && editor.value?.getHTML() === "<p></p>") {
    emit("deleteIfEmpty");
  } else if (
    event.key === "ArrowUp" &&
    editor.value?.state.selection.$from.parentOffset === 0 &&
    editor.value.state.selection.$from.nodeBefore == null
  ) {
    emit("navigateUp");
  } else if (
    event.key === "ArrowDown" &&
    editor.value?.state.selection.$from.parentOffset === editor.value?.state.selection.$from.parent.content.size
  ) {
    emit("navigateDown");
  } else if (event.key === "Escape") {
    emit("escape");
    blur();
  }
}

// TODO @Broken @UX: programmatically apply apperance classes to tiptap editor
const appearance = useAppearance();
const editor = useEditor({
  content: props.modelValue,
  extensions: [StarterKit, shortcutsExtension],
  editorProps: {
    attributes: {
      class: "prose prose-h1:text-3xl prose-h2:text-xl prose-a:text-gray-700 w-full text-sm",
    },
  },
  editable: !props.readonly,
  onUpdate({ editor }) {
    emit("update:modelValue", editor.getHTML());
  },
  onFocus() {
    focused.value = true;
  },
  onBlur() {
    focused.value = false;
  },
});

// sync modelValue into editor
watch(
  () => props.modelValue,
  (value: string) => {
    if (value !== editor.value?.getHTML()) {
      editor.value?.commands.setContent(value);
    }
  }
);

// handle focus
function focus(end?: boolean) {
  editor.value?.commands.focus(end ? "end" : "start");
}
function blur() {
  editor.value?.commands.blur();
}

defineExpose({
  focus,
  blur,
  focused,
});
</script>
<template>
  <EditorContent :editor="editor" @keydown="onKeyDown" />
</template>
<style>
.ProseMirror:focus {
  outline: none;
}
</style>
