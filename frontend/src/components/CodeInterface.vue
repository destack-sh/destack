<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
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
const statement = computed(() => useFragment(StatementContentType, props.statement));

const operations = useOperations();
const monacoEditorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);
const code: Ref<string | null> = ref(null);

function saveCode(code: string) {
  const oldCode = statement.value.code ?? "";
  if (oldCode !== code) {
    operations.content.updateStatementCode(statement.value.id, { code: oldCode }, { code });
  }
}
const saveCodeDebounced = useDebounceFn(saveCode, 200, { maxWait: 500 });

// sync code to local if not editing
watchEffect(() => {
  if (!props.editing || code.value == null) {
    code.value = statement.value.code ?? "";
  }
});

defineExpose({
  focus: () => monacoEditorRef.value?.focus(),
  defocus: () => monacoEditorRef.value?.defocus(),
});
</script>
<template>
  <MonacoEditor
    ref="monacoEditorRef"
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
    :readonly="props.readonly"
  />
</template>
