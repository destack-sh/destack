<script lang="ts" setup>
const props = defineProps<{
  label?: string | null;
  sublabel?: string | null;
  sublabelPosition?: "adjacent" | "opposite";
}>();
</script>
<template>
  <div class="flex flex-col gap-y-0.5">
    <!-- Label above -->
    <div
      v-if="label"
      class="flex w-full flex-row px-2 text-sm font-semibold"
      :class="[sublabelPosition == 'opposite' ? 'justify-between' : '']"
    >
      {{ label }}
      <span class="inline-flex flex-row gap-2 font-normal text-gray-400">
        <span v-if="sublabel" class="ml-1">{{ sublabel }}</span>
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
