<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useNotifications } from "@/state/notifications";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { BellIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const notifications = useNotifications();
const hasUnreadNotifications = computed(() => false);
</script>
<template>
  <Popover v-slot="{ open }" as="div" class="relative">
    <PopoverButton
      class="rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-gray-500 hover:bg-orange-50': !hasUnreadNotifications,
        'text-orange-600 hover:bg-orange-50': hasUnreadNotifications,
        'bg-orange-50': open,
      }"
    >
      <BellIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-10 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pt-2 pb-4 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div>
          <h2 class="font-bold text-gray-900">Notifications</h2>
          <!-- Unread / archived toggle -->
          <!-- Dismiss all button -->
        </div>

        <p class="text-gray-500">Nothing here.</p>
        <!-- Notifications -->
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
