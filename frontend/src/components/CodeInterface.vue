<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/state/code";
import { StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementHeaderType>;
  content: FragmentType<typeof CodeContentType>;
  focused: boolean;
  editing: boolean;
  readonly: boolean;
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

const operations = useOperations();
const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);
const code = ref("");

function saveCode(code: string) {
  const oldCode = content.value.code ?? "";
  if (oldCode !== code) {
    operations.content.updateCodeContent(statement.value.id, { code: oldCode }, { code });
  }
}
const saveCodeDebounced = useDebounceFn(saveCode, 200, { maxWait: 500 });

// sync code to local if not editing
watchEffect(() => {
  if (!props.editing) {
    code.value = content.value.code ?? "";
  }
});

defineExpose({
  focus: () => monacoEditor.value?.focus(),
  defocus: () => monacoEditor.value?.defocus(),
});
</script>
<template>
  <MonacoEditor
    ref="monacoEditor"
    v-if="code != null"
    :line-number-offset="lineNumberBase + 1 /* for statement itself */"
    :line-number-shift-px="xOffset + 20"
    :style="{ marginLeft: -xOffset - 43 + 'px' }"
    :model-value="code"
    @update:model-value="saveCodeDebounced"
    @navigateUp="emit('navigateUp')"
    @navigateDown="emit('navigateDown')"
    @escape="emit('escape')"
    language="python"
    :focused="focused"
    :readonly="readonly"
  />
</template>
