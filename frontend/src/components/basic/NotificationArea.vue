<script lang="ts" setup>
import { useNotifications, type DisplayNotification } from "@/state/notifications";
import { XMarkIcon } from "@heroicons/vue/24/outline";
import { CheckCircleIcon, ExclamationCircleIcon, InformationCircleIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const notifications = useNotifications();

const MAX_NOTIFICATIONS = 3;

const shownNotifications = computed(() => notifications.shownNotifications.value.slice(0, MAX_NOTIFICATIONS));

function getIcon(notification: DisplayNotification) {
  if (notification.icon) {
    return notification.icon;
  }
  // default to icons by level
  const iconByKind = {
    error: XCircleIcon,
    notice: InformationCircleIcon,
    warning: ExclamationCircleIcon,
    success: CheckCircleIcon,
  };
  return iconByKind[notification.kind];
}

function freezeNotification(notification: DisplayNotification) {
  notifications.store.freeze(notification.localId);
}
</script>

<template>
  <div class="pointer-events-none fixed inset-0 z-40 flex items-start px-4 py-2 sm:px-6 sm:py-3">
    <div class="flex h-full w-full flex-col items-center gap-y-1.5 transition">
      <transition-group
        move-class="transition-all"
        enter-active-class="transition duration-100 ease-out transform"
        enter-from-class="translate-y-0 translate-y-2 opacity-0"
        enter-to-class="translate-y-0 opacity-100"
        leave-active-class="absolute transition duration-100 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
        appear
      >
        <!-- Message body-->
        <div
          v-for="notification in shownNotifications"
          :key="notification.localId"
          class="pointer-events-auto flex w-[400px] flex-row overflow-hidden rounded-sm bg-white px-3 shadow-md ring-1 ring-orange-900 ring-opacity-[12%]"
          @mouseenter="freezeNotification(notification)"
        >
          <!-- Main message -->
          <div class="my-2.5 flex max-w-full flex-1 flex-row truncate text-sm">
            <h3
              class="text-sm font-semibold"
              :class="{
                'text-green-700': notification.kind === 'success',
                'text-gray-700': notification.kind === 'notice',
                'text-red-600': notification.kind === 'error',
                'text-yellow-600': notification.kind === 'warning',
              }"
            >
              {{ notification.message }}
            </h3>
            <span v-if="notification.description" class="ml-1.5 truncate text-gray-400">
              {{ notification.description }}
            </span>
          </div>
          <!-- Actions -->
          <button
            v-if="notification.action"
            class="h-full min-w-0 flex-shrink-0 self-center rounded-r-sm px-3 py-1 text-sm font-medium text-orange-500 hover:bg-orange-100 hover:decoration-gray-900 focus:outline-none"
            @click="() => (notification.action?.(), notifications.dismiss(notification.localId))"
          >
            {{ notification.actionText }}
          </button>
        </div>
      </transition-group>
    </div>
  </div>
</template>
