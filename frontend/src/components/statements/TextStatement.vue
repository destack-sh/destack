<script lang="ts" setup>
import TiptapEditor from "@/components/basic/TiptapEditor.vue";
import { useStatementContext } from "@/state/statement";
import { EllipsisHorizontalIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

const context = useStatementContext();
const editorRef = ref<InstanceType<typeof TiptapEditor> | null>(null);
const content: Ref<string> = ref(context.statement.value.code ?? "");
context.syncText(
  content,
  computed(() => editorRef.value?.focused)
);

function countWordsInHtml(html: string): number {
  // Remove HTML tags and special characters
  const cleanText = html.replace(/<[^>]*>/g, "").replace(/&[^;]+;/g, "");

  // Split the text into words and filter out empty strings
  const words = cleanText.split(/\s+/).filter((word) => word !== "");

  // Return the count of words
  return words.length;
}
const numWords = computed(() => countWordsInHtml(content.value));

function focus(position: "first" | "last" = "first") {
  // focus the end of the content if we just updated it, which puts it in pending state
  // (likely due to a morph to blank where we want to keep editing smoothly)
  const focusEnd = context.statement.value.revision < 0 || position == "last";
  // not sure why we need both, but acquiring focus doesn't always succeed otherwise
  editorRef.value?.focus(focusEnd);
  nextTick(() => editorRef.value?.focus(focusEnd));
}

// morph back to blank if it's empty for smooth back and forth
watch(content, () => {
  if (content.value.trim() == "<p></p>") {
    context.morphToBlank();
  }
});

defineExpose({
  focus,
  blur: () => editorRef.value?.blur(),
});
</script>
<template>
  <button
    v-if="folded"
    class="ml-1 flex max-w-full flex-row items-center gap-1.5 truncate rounded-sm px-0.5 text-gray-400 hover:bg-gray-100"
    @click="emit('toggleFold')"
  >
    <EllipsisHorizontalIcon class="h-4 w-4" />
    <span v-if="numWords > 0">{{ numWords }} {{ numWords == 1 ? "word" : "words" }}</span>
  </button>
  <TiptapEditor
    v-else
    ref="editorRef"
    :model-value="content || ''"
    @update:model-value="content = $event"
    @navigateUp="context.navigateUp"
    @navigateDown="context.navigateDown"
    @escape="context.escape"
    @enter="context.insertBelow"
    @delete-if-empty="context.deleteSelf"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class=""
  />
</template>
