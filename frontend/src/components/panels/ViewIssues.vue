<script lang="ts" setup>
import type { InterpError } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { fileOf, useCurrentModuleRuntime } from "@/state/runtime";
import { XCircleIcon } from "@heroicons/vue/24/outline";

const runtime = useCurrentModuleRuntime();
const editor = useEditorState();

function focusError(error: InterpError) {
  console.log("focus error", error);
  if (error.symbol != null) {
    const file = fileOf(error.symbol);
    if (!file) return;
    editor.focusFile(file as any);
    editor.editElement(error.symbol as any);
  }
}
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex h-[31px] flex-row items-center justify-between border-b border-gray-200 px-3 py-2">
      <span class="text-xs font-bold uppercase">Issues</span>
    </div>
    <ul v-for="(error, i) in runtime.errors.value ?? []" :key="i" class="py-2">
      <li
        class="group flex flex-col justify-between gap-x-1 py-1 text-sm hover:cursor-pointer hover:bg-orange-50"
        @click="focusError(error as InterpError)"
      >
        <div v-if="error.symbol != null" class="px-3">
          <span class="text-gray-900 group-hover:text-orange-600"
            >{{ fileOf(error.symbol)?.path }}.{{ error.symbol.name }}</span
          >
        </div>
        <span class="flex flex-row items-center gap-1 px-3 text-red-600">
          <XCircleIcon class="h-4 w-4" />
          <span>{{ error.message }}</span>
        </span>
      </li>
    </ul>
  </div>
</template>
