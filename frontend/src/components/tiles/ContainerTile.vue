<script lang="ts" setup>
const props = defineProps<{
  label?: string | null;
  subLabel?: string | null;
}>();
</script>
<template>
  <div class="flex flex-col gap-y-0.5">
    <!-- Label above -->
    <div v-if="label" class="w-full flex-row justify-between px-2 text-sm font-semibold">
      {{ label }}
      <span class="inline-flex flex-row gap-2 font-normal text-gray-400">
        <span v-if="subLabel" class="ml-1">{{ subLabel }}</span>
        <slot name="sublabel" class="ml-1" />
      </span>
    </div>
    <div
      class="flex h-full w-full flex-1 flex-col rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-2 shadow-sm"
    >
      <!-- Content -->
      <slot />
    </div>
  </div>
</template>
