<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  label?: string;
  labelPosition?: "top" | "above";
}>();
const labelPosition = computed(() => props.labelPosition ?? "above");
</script>
<template>
  <div class="z-[1] flex flex-col gap-y-0.5">
    <span v-if="label && labelPosition == 'above'" class="px-2 text-sm font-bold">{{ label }}</span>
    <div
      class="flex h-full w-full flex-1 flex-col rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-2 shadow-sm"
    >
      <span v-if="label && labelPosition == 'top'" class="text-sm font-bold">{{ label }}</span>
      <slot />
    </div>
  </div>
</template>
