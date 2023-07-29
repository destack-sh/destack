<script lang="ts" setup>
import { useEditor, BubbleMenu, EditorContent, Extension } from "@tiptap/vue-3";
import StarterKit from "@tiptap/starter-kit";
import { ref, watch, watchEffect, type Ref } from "vue";
import { useAppearance } from "@/state/appearance";
import { BubbleMenu as BubbleMenuExt } from "@tiptap/extension-bubble-menu";

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
  (e: "enterStart"): void;
  (e: "deleteStart"): void;
  (e: "enter"): void;
  (e: "execute"): void;
}>();

const focused: Ref<boolean> = ref(false);

// custom shortcuts
const shortcutsExtension = Extension.create({
  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        if (editor.state.selection.$from.pos === 1) {
          emit("enterStart");
          return true;
        }
        return false;
      },
      Backspace: ({ editor }) => {
        if (editor.state.selection.$from.pos === 1) {
          emit("deleteStart");
          return true;
        }
        return false;
      },
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

const appearance = useAppearance();
function getEditorClass(): string {
  const classes = ["prose prose-h1:text-3xl prose-h2:text-xl  prose-a:text-gray-500 w-full"];
  if (appearance.fontMono) {
    classes.push("font-mono");
  }
  if (appearance.textSmall) {
    classes.push("text-sm");
  } else {
    classes.push("text-md");
  }
  return classes.join(" ");
}

const editor = useEditor({
  content: props.modelValue,
  extensions: [
    StarterKit,
    shortcutsExtension,
    BubbleMenuExt.configure({
      element: document.querySelector(".menu"),
    }),
  ],
  parseOptions: {
    preserveWhitespace: "full",
  },
  editorProps: {
    attributes: {
      class: getEditorClass(),
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

// sync appearance changes into editor
watchEffect(() => {
  editor.value?.setOptions({
    editorProps: {
      attributes: {
        class: getEditorClass(),
      },
    },
  });
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
  <div>
    <BubbleMenu
      :editor="editor"
      :tippy-options="{ duration: 100 }"
      v-if="editor"
      class="z-20 flex flex-row gap-1 rounded-sm border border-orange-900 border-opacity-20 bg-white p-1"
    >
      <button
        @click="editor.chain().focus().toggleBold().run()"
        class="text-sm hover:bg-orange-100"
        :class="{ 'font-bold text-gray-900': editor.isActive('bold'), 'text-gray-500': !editor.isActive('bold') }"
      >
        bold
      </button>
      <button
        @click="editor.chain().focus().toggleItalic().run()"
        class="text-sm hover:bg-orange-100"
        :class="{ 'font-bold text-gray-900': editor.isActive('italic'), 'text-gray-500': !editor.isActive('italic') }"
      >
        italic
      </button>
      <button
        @click="editor.chain().focus().toggleStrike().run()"
        class="text-sm hover:bg-orange-100"
        :class="{ 'font-bold text-gray-500': editor.isActive('strike'), 'text-gray-500': !editor.isActive('strike') }"
      >
        strike
      </button>
    </BubbleMenu>
    <EditorContent :editor="editor" @keydown="onKeyDown" />
  </div>
</template>
<style>
.ProseMirror:focus {
  outline: none;
}
</style>
