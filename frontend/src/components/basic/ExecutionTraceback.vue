<script lang="ts" setup>
import { ExecutionStatus, type Execution } from "@/gql/graphql";

const props = defineProps<{
  name: string;
  execution: Execution;
}>();
</script>
<template>
  <div class="relative w-full" :class="[execution.status == ExecutionStatus.Failed ? 'text-red-600' : 'text-gray-600']">
    {{ name }} {{ execution.status.toLowerCase() }}:
    <span class="font-bold">{{ execution.errorNice?.message }}</span>
    <ul class="mt-1 flex flex-col gap-2">
      <!-- Error traceback -->
      <li
        v-for="(frame, i) of execution.errorNice?.traceback"
        :key="i"
        class="flex max-w-full flex-col overflow-hidden py-0.5 hover:bg-red-100"
      >
        <span>
          <a class="underline underline-offset-4">{{ frame.filename }}:{{ frame.lineno }}</a> {{ frame.name }}
        </span>
        <span class="mx-2 mt-0.5" :class="i == 0 ? 'font-bold' : ''"> > {{ frame.line }} </span>
        <!-- Locals -->
        <span
          v-if="Object.keys(frame.locals).length > 0"
          class="mx-2 mt-0.5 grid grid-cols-4 border border-red-600 p-2"
        >
          <template v-for="key in Object.keys(frame.locals)" :key="key">
            <span>{{ key }}</span>
            <span class="col-span-3 w-full">{{ frame.locals[key] }}</span>
          </template>
        </span>
      </li>
    </ul>
    <slot />
  </div>
</template>
