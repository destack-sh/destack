<script lang="ts" setup>
import { useNotifications } from "@/state/notifications";
import { XMarkIcon } from "@heroicons/vue/24/outline";

const notifications = useNotifications();
</script>

<template>
  <div class="pointer-events-none fixed inset-0 z-40 flex items-start px-4 py-6 sm:p-6">
    <div class="flex h-full w-full flex-col-reverse items-center gap-y-4 transition">
      <transition-group
        move-class="transition-all"
        enter-active-class="transition duration-100 ease-out transform"
        enter-from-class="translate-y-2 opacity-0 translate-y-0"
        enter-to-class="translate-y-0 opacity-100"
        leave-active-class="absolute transition duration-50 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
        appear
      >
        <div
          v-for="notification in notifications.activeNotifications"
          :key="notification.id"
          class="pointer-events-auto flex w-full max-w-sm items-center overflow-hidden rounded-sm p-3 shadow-sm ring-1 ring-black ring-opacity-5"
          :class="{
            'bg-white': notification.kind != 'error',
            'bg-red-100': notification.kind == 'error',
          }"
        >
          <div class="flex w-0 flex-1 justify-between">
            <p class="w-0 flex-1 text-sm font-medium text-gray-900">{{ notification.message }}</p>
            <button
              v-if="notification.actionText"
              type="button"
              class="ml-3 flex-shrink-0 rounded-md text-sm font-medium text-orange-600 hover:text-orange-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
              @click="() => (notification.action?.(), notifications.dismiss(notification.id))"
            >
              {{ notification.actionText }}
            </button>
          </div>
          <div class="ml-4 flex flex-shrink-0">
            <button
              type="button"
              @click="notifications.dismiss(notification.id)"
              class="inline-flex rounded-md text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            >
              <span class="sr-only">Close</span>
              <XMarkIcon class="h-5 w-5" aria-hidden="true" />
            </button>
          </div>
        </div>
      </transition-group>
    </div>
  </div>
</template>
