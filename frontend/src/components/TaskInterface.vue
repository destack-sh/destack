<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";
import { useEditorState } from "@/utils/editor";
import { StatementContentType } from "@/utils/fragments";
import { useOperations } from "@/utils/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, type Ref } from "vue";

const TaskContentType = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    description
  }
`);

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof TaskContentType>;
  compiled: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
}>();

const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(TaskContentType, props.content));

const editor = useEditorState();
const readonly = computed(() => editor.readonly || props.compiled);
const operations = useOperations();

const description: Ref<HTMLInputElement | null> = ref(null);
async function onDescriptionEnter() {
  const newDescription = (description.value as HTMLInputElement).innerText;
  if (newDescription.length > 0 && newDescription != content.value.description) {
    await operations.content.updateTaskContent(statement.value.id, content.value.description, newDescription);
  }
}

const onDescriptionEnterDebounced = useDebounceFn(onDescriptionEnter, 200);

function focus() {
  description.value?.focus();
}
function defocus() {
  description.value?.blur();
}

defineExpose({ focus, defocus });
</script>
<template>
  <div class="flex flex-col text-sm text-black">
    <span
      ref="description"
      :contenteditable="!readonly"
      maxlength="200"
      class="inline w-full rounded-sm bg-transparent py-0.5 text-sm text-inherit placeholder-gray-400 outline-none hover:bg-yellow-50 focus:bg-yellow-100"
      @keydown.enter.prevent="onDescriptionEnter"
      @keydown.up.prevent="emit('navigateUp', description?.selectionStart as number)"
      @keydown.down.prevent="emit('navigateDown', description?.selectionStart as number)"
      @keydown.esc.prevent="emit('escape')"
      @keydown="onDescriptionEnterDebounced"
    >
      {{ content.description }}
    </span>
  </div>
</template>
