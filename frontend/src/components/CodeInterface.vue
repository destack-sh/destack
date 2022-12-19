<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/utils/code";
import { useEditorState } from "@/utils/editor";
import { StatementHeaderType } from "@/utils/fragments";
import { useOperations } from "@/utils/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementHeaderType>;
  content: FragmentType<typeof CodeContentType>;
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
const statement = computed(() => useFragment(StatementHeaderType, props.statement));
const content = computed(() => useFragment(CodeContentType, props.content));

const editor = useEditorState();
const readonly = computed(() => editor.readonly || props.compiled);
const operations = useOperations();

function onCodeEnter(code: string) {
  const oldCode = content.value.code ?? "";
  if (oldCode !== code) {
    operations.content.updateCodeContent(statement.value.id, { code: oldCode }, { code });
  }
}

const onCodeEnterDebounced = useDebounceFn(onCodeEnter, 200, { maxWait: 500 });

const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);

function focus() {
  monacoEditor.value?.focus();
}

function defocus() {
  monacoEditor.value?.defocus();
}

defineExpose({ focus, defocus });
</script>
<template>
  <MonacoEditor
    ref="monacoEditor"
    v-if="content.code"
    :line-number-offset="lineNumberBase + 1 /* for statement itself */"
    :line-number-shift-px="xOffset + 20"
    :style="{ marginLeft: -xOffset - 44 + 'px' }"
    :model-value="content.code"
    @update:model-value="onCodeEnterDebounced"
    @navigateUp="emit('navigateUp')"
    @navigateDown="emit('navigateDown')"
    @escape="emit('escape')"
    language="python"
    :focused="focused"
    :readonly="readonly"
  />
</template>
