<script lang="ts" setup>
import type { InterpError } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { fileOf, useVisibleErrors } from "@/state/runtime";
import { XCircleIcon } from "@heroicons/vue/24/outline";

const editor = useEditorState();
const errors = useVisibleErrors();

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
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
    >
      <span class="text-xs font-bold uppercase">Issues</span>
    </div>
    <ul class="flex flex-col gap-2 py-2">
      <li
        v-for="(error, i) in errors ?? []"
        :key="i"
        class="group flex flex-col justify-between py-0.5 text-sm hover:cursor-pointer hover:bg-orange-50"
        @click="focusError(error as InterpError)"
      >
        <div v-if="error.symbol != null" class="px-3">
          <span class="text-gray-700">{{ SYMBOL_TYPE_KEYWORD[error.symbol.symbolType] }}</span>
          <span class="pl-1 text-gray-900 group-hover:text-orange-600"
            >{{ fileOf(error.symbol)?.path }}.{{ error.symbol.name }}</span
          >
        </div>
        <span class="flex flex-row gap-1 px-3 text-red-600">
          <XCircleIcon class="mt-0.5 h-4 w-4" />
          <span>{{ error.message }}</span>
        </span>
      </li>
    </ul>
  </div>
</template>
