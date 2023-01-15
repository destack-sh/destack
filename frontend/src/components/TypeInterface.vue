<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { TypeContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watchEffect, type Ref } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementHeaderType>;
  content: FragmentType<typeof TypeContentType>;
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
const content = computed(() => useFragment(TypeContentType, props.content));

const operations = useOperations();
const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);
const btl: Ref<string | null> = ref(null);

function saveBtl(btl: string) {
  const oldBtl = content.value.btl ?? "";
  if (oldBtl !== btl) {
    operations.content.updateTypeContent(statement.value.id, oldBtl, btl);
  }
}
const saveBtlDebounced = useDebounceFn(saveBtl, 200, { maxWait: 500 });

// sync btl to local if not editing
watchEffect(() => {
  if (!props.editing || btl.value == null) {
    btl.value = content.value.btl ?? "";
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
      :model-value="btl"
      @update:model-value="saveBtlDebounced"
      language="btl"
      :focused="focused"
      :readonly="readonly"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
  </div>
</template>
