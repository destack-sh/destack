<script lang="ts" setup>
import MonacoEditor from "@/components/basic/MonacoEditor.vue";
import { computed, ref, watchEffect, type Ref } from "vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { SparklesIcon as SparklesIconSolid } from "@heroicons/vue/24/solid";
import { SparklesIcon as SparklesIconOutline } from "@heroicons/vue/24/outline";
import { useNow } from "@/composables/useNow";
import { DateTime } from "luxon";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const now = useNow(10000);
const highlightAssist = ref(true);
const code: Ref<string> = ref(props.statement.code ?? "");
watchEffect(() => {
  // highlight if statement just created or code is very short
  if (now.value.diff(DateTime.fromISO(props.statement.createdAt)).as("minutes") > 10 || code.value.length >= 30) {
    highlightAssist.value = false;
  }
});

const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
const codeSync = syncProperty({
  read: () => (code.value = props.statement.code ?? ""),
  write: () => {
    monacoRef.value?.markPosition();
    ops.symbol.updateSymbolCode(null, props.statement.id, props.statement.code ?? "", code.value);
  },
  debounceMs: 500,
  debounceMaxWait: 5000,
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    monacoRef.value?.focus(position);
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
  <div class="relative">
    <MonacoEditor
      ref="monacoRef"
      v-model="code"
      @update:model-value="codeSync.onLocalWrite"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @escape="emit('escape')"
      @enter="emit('enter')"
      @execute="emit('run')"
      @open-actions="emit('openActions')"
      language="python"
      wrap
      :focused="focused"
      :readonly="readonly"
      class="-mx-1 mt-0.5 min-h-[32px] rounded-t-sm border border-orange-900 border-opacity-[15%] px-1 pb-1.5 pt-1 transition-colors duration-150"
    />
    <!-- Assist button -->
    <button
      v-if="!readonly"
      class="group absolute right-3 top-1.5 flex flex-row rounded-sm px-1 py-0.5 transition-all duration-150"
      :class="[
        !focused ? 'text-gray-400 hover:bg-orange-100' : '',
        focused && !highlightAssist ? ' text-orange-600 hover:bg-orange-100' : '',
        focused && highlightAssist ? ' bg-orange-600 text-white' : '',
      ]"
      @click="emit('launchAssist', 'Implement this')"
    >
      <component :is="focused ? SparklesIconSolid : SparklesIconOutline" class="mt-0.5 h-4 w-4" />
      <span v-if="focused && highlightAssist" class="ml-1 font-semibold">Assist</span>
      <!-- Label -->
      <span
        class="pointer-events-none absolute right-0 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100 group-hover:delay-in-500"
      >
        Help write the code
      </span>
    </button>
  </div>
</template>
