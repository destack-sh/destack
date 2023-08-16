<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { Popover, PopoverPanel } from "@headlessui/vue";

defineProps<{ title?: string; description?: string; confirmText?: string; cancelText?: string }>();
const emit = defineEmits<{ (e: "action"): void; (e: "cancel"): void }>();
</script>
<template>
  <Popover class="relative" v-slot="{ open, close }">
    <slot :open="open" />
    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-6 z-10 mt-0 flex w-60 flex-col gap-4 rounded-sm bg-white p-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div class="px-1.5">
          <h2 class="text-left font-bold text-gray-900">{{ title ?? "Confirm" }}</h2>
          <p class="pt-0.5 text-left text-gray-900">{{ description ?? "Are you sure you want this?" }}</p>
        </div>
        <!-- Actions -->
        <div class="flex w-full flex-row items-center justify-between gap-2">
          <button
            @click="
              close();
              emit('cancel');
            "
            class="rounded-sm px-1.5 py-0.5 text-gray-700 hover:bg-orange-100"
          >
            {{ cancelText ?? "Cancel" }}
          </button>
          <button
            @click="
              close();
              emit('action');
            "
            class="rounded-sm px-1.5 py-0.5 text-gray-900 hover:bg-orange-100"
          >
            {{ confirmText ?? "Confirm" }}
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
