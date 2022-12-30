<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { SchemaContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementHeaderType>;
  content: FragmentType<typeof SchemaContentType>;
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
const content = computed(() => useFragment(SchemaContentType, props.content));

const operations = useOperations();
const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);
const bsl: Ref<string | null> = ref(null);

function saveBsl(bsl: string) {
  const oldBsl = content.value.bsl ?? "";
  if (oldBsl !== bsl) {
    operations.content.updateSchemaContent(statement.value.id, oldBsl, bsl);
  }
}
const saveBslDebounced = useDebounceFn(saveBsl, 200, { maxWait: 500 });

// sync bsl to local if not editing
watchEffect(() => {
  if (!props.editing || bsl.value == null) {
    bsl.value = content.value.bsl ?? "";
  }
});

defineExpose({
  focus: () => monacoEditor.value?.focus(),
  defocus: () => monacoEditor.value?.defocus(),
});
</script>
<template>
  <div class="flex h-full w-full flex-col gap-1 text-sm">
    <span v-if="content.description" class="text-black">{{ content.description }}</span>
    <MonacoEditor
      ref="monacoEditor"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      hide-line-numbers
      :style="{ marginLeft: -23 + 'px' }"
      :model-value="bsl"
      @update:model-value="saveBslDebounced"
      language="bsl"
      :focused="focused"
      :readonly="readonly"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
  </div>
</template>
