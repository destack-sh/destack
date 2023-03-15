<script lang="ts" setup>
import type { InterpError } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { fileOf, useSymbolNavigation, useVisibleErrors } from "@/state/runtime";
import { FaceSmileIcon, XCircleIcon } from "@heroicons/vue/24/outline";

const editor = useEditorState();
const errors = useVisibleErrors();
const { focusSymbol } = useSymbolNavigation();

function focusError(error: InterpError) {
  console.log("focus error", error);
  if (error.symbol != null) {
    focusSymbol(error.symbol);
  }
}
</script>
<template>
  <div class="">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
    >
      <span class="text-xs font-bold uppercase">
        Issues
        <span class="ml-1 rounded-lg bg-gray-200 px-1 font-normal text-gray-800" v-if="errors.length">
          {{ errors.length }}
        </span>
      </span>
    </div>
    <ul class="flex w-full flex-col gap-2 overflow-y-auto py-2 pb-10">
      <li
        v-for="(error, i) in errors ?? []"
        :key="i"
        class="group flex flex-col justify-between py-0.5 text-sm hover:cursor-pointer hover:bg-orange-100"
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
      <div v-if="errors.length == 0" class="my-4 flex flex-col items-center justify-center gap-2 px-3 text-center">
        <FaceSmileIcon class="h-7 w-7 text-gray-500" />
        <span class="text-sm text-gray-700">A tidy Bench. The bots like it.</span>
      </div>
    </ul>
  </div>
</template>
