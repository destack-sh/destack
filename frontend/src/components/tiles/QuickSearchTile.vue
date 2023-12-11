<script lang="ts" setup>
import { MagnifyingGlassIcon } from "@heroicons/vue/24/outline";
import { ref } from "vue";

const props = defineProps<{
  modelValue?: string;
  placeholder?: string;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const inputRef = ref<HTMLInputElement | null>(null);
</script>
<template>
  <div
    class="flex flex-row items-center rounded-xl px-0.5 py-0.5 ring-1 ring-amber-900/[12%] focus-within:ring-amber-300"
  >
    <button class="px-0.5 py-0.5 hover:bg-amber-100" @click="inputRef?.focus()">
      <MagnifyingGlassIcon class="h-4 w-4 text-gray-900" />
    </button>
    <input
      ref="inputRef"
      type="text"
      class="ml-1 flex-grow border-none bg-transparent px-0.5 py-0 text-sm text-gray-900 outline-none ring-0 placeholder:text-gray-400 focus:ring-0"
      :placeholder="placeholder"
      :value="modelValue"
      @input="($event) => emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
  </div>
</template>
