<script lang="ts" setup>
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { useCurrentModuleRuntime } from "@/state/runtime";

const runtime = useCurrentModuleRuntime();
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex h-[31px] flex-row items-center justify-between border-b border-gray-200 px-3 py-2">
      <span class="text-xs font-bold uppercase">Issues</span>
    </div>
    <ul v-for="(error, i) in runtime.errors.value ?? []" :key="i" class="py-2">
      <li class="flex flex-col justify-between gap-x-1 py-1 text-sm">
        <div v-if="error.symbol" class="flex flex-row gap-1 px-3">
          <span class="text-gray-500">{{ SYMBOL_TYPE_KEYWORD[error.symbol.symbolType] }}</span>
          <span class="">{{ error.symbol.name }}</span>
        </div>
        <span class="px-3 text-red-600">
          {{ error.message }}
        </span>
      </li>
    </ul>
  </div>
</template>
