<script lang="ts" setup>
import type { RunError } from "@/gql/graphql";
import { useCurrentModule } from "@/state/module";
import { computed } from "vue";

const props = defineProps<{
  errorNice: RunError;
  statementCk?: string;
  hidePreamble?: boolean;
}>();
const module = useCurrentModule();
const statementName = computed(() => (props.statementCk == null ? null : module.statementOf(props.statementCk)?.name));
</script>
<template>
  <div class="relative w-full font-mono text-red-600">
    <span class="whitespace-pre-wrap font-bold">{{ errorNice?.type }}: {{ errorNice?.message }}</span>
    <ul class="mt-1 flex flex-col gap-2">
      <!-- Error traceback -->
      <li
        v-for="(frame, i) of errorNice?.traceback"
        :key="i"
        class="flex max-w-full flex-col overflow-hidden py-0.5 hover:bg-red-100"
      >
        <span>
          <a class="underline underline-offset-4">{{ frame.filename }}:{{ frame.lineno }}</a> {{ frame.name }}
        </span>
        <span class="mx-2 mt-0.5" :class="i == 0 ? 'font-semibold' : ''"> > {{ frame.line }} </span>
        <!-- Locals -->
        <span
          v-if="Object.keys(frame.locals ?? {}).length > 0"
          class="mx-2 mt-0.5 grid grid-cols-4 border border-red-600 p-2"
        >
          <template v-for="key in Object.keys(frame.locals)" :key="key">
            <span>{{ key }}</span>
            <span class="scroll-hidden col-span-3 max-h-40 w-full overflow-y-scroll">{{ frame.locals[key] }}</span>
          </template>
        </span>
      </li>
    </ul>
  </div>
</template>
