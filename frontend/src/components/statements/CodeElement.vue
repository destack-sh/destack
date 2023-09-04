<script lang="ts" setup>
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import { nextTick, computed, ref, type Ref, watch } from "vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();

const code: Ref<string> = ref(props.statement.code ?? "");
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
const codeSync = syncProperty({
  value: code,
  editing: computed(() => monacoRef.value?.focused),
  read: () => (code.value = props.statement.code ?? ""),
  write: () => ops.symbol.updateSymbolCode(null, props.statement.id, props.statement.code ?? "", code.value),
  debounceMs: 500,
  debounceMaxWait: 5000,
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    monacoRef.value?.focus(position == "last");
  },
  blur: () => {
    monacoRef.value?.blur();
  },
  syncNow: () => {
    codeSync.flushNow();
  },
});
</script>
<template>
  <!-- Code -->
  <!-- TODO @UX: figure out nicer styling for code -->
  <MonacoEditor
    ref="monacoRef"
    :hide-line-numbers="false"
    v-model="code"
    @navigate-up="emit('navigateUp')"
    @navigate-down="emit('navigateDown')"
    @navigate-left="emit('navigateLeft')"
    @escape="emit('escape')"
    @enter="emit('enter')"
    @execute="emit('run')"
    @toggle-actions="emit('openActions')"
    language="python"
    :focused="focused"
    :readonly="readonly"
    class="-mx-1 mt-0.5 min-h-[32px] rounded-t-sm border border-orange-900 border-opacity-[15%] px-1 pb-1.5 pt-1 transition-colors duration-150"
  />
</template>
