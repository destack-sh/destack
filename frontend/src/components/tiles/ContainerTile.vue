<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  label?: string | null;
  subLabel?: string | null;
  labelPosition?: "top" | "above";
}>();
const labelPosition = computed(() => props.labelPosition ?? "above");
</script>
<template>
  <div class="flex flex-col gap-y-0.5">
    <!-- Label above -->
    <span v-if="label && labelPosition == 'above'" class="px-2 text-sm font-semibold"
      >{{ label }}
      <span v-if="subLabel" class="ml-1 font-light text-gray-400">{{ subLabel }}</span>
    </span>
    <div
      class="flex h-full w-full flex-1 flex-col rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-2 shadow-sm"
    >
      <!-- Label inside on top -->
      <span v-if="label && labelPosition == 'top'" class="text-sm font-semibold"
        >{{ label }}
        <span v-if="subLabel" class="ml-1 font-light text-gray-400">{{ subLabel }}</span>
      </span>
      <!-- Content -->
      <slot />
    </div>
  </div>
</template>
