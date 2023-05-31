<script lang="ts" setup>
import { useNotifications, type DisplayNotification } from "@/state/notifications";
import { CheckCircleIcon, ExclamationCircleIcon, InformationCircleIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const notifications = useNotifications();

const MAX_NOTIFICATIONS = 4;

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
  <div class="pointer-events-none fixed inset-0 z-40 flex items-start px-4 py-6 sm:p-6">
    <div class="flex h-full w-full flex-col-reverse items-center gap-y-4 transition">
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
          class="pointer-events-auto flex w-[400px] flex-row overflow-hidden rounded-sm bg-white shadow-md ring-1 ring-orange-900 ring-opacity-[12%]"
          @mouseenter="freezeNotification(notification)"
        >
          <!-- Icon -->
          <div class="m-3">
            <component
              :is="getIcon(notification)"
              class="h-5 w-5"
              :class="{
                'text-green-700': notification.kind === 'success',
                'text-gray-400': notification.kind === 'notice',
                'text-red-600': notification.kind === 'error',
                'text-yellow-600': notification.kind === 'warning',
              }"
            />
          </div>
          <!-- Main message -->
          <div class="m-3 ml-0 flex max-w-full flex-1 flex-col">
            <h3 class="text-sm font-bold text-gray-900">{{ notification.message }}</h3>
            <p v-if="notification.description" class="min-w-0 max-w-full pt-0.5 text-sm text-gray-500">
              {{ notification.description }}
            </p>
          </div>
          <!-- Actions -->
          <div
            v-if="notification.action"
            class="flex flex-shrink-0 flex-col border-l border-orange-900 border-opacity-[12%]"
          >
            <button
              class="h-full min-w-0 self-center rounded-r-sm px-3 py-1 text-sm font-medium text-orange-500 hover:bg-orange-100 hover:decoration-gray-900 focus:outline-none"
              @click="() => (notification.action?.(), notifications.dismiss(notification.localId))"
            >
              {{ notification.actionText }}
            </button>
            <!-- Dismiss? -->
            <!-- <button
              class="h-full min-w-0 self-center rounded-r-sm text-sm text-gray-500"
              @click="notifications.store.dismiss(notification.id as string)"
            >
              Dismiss
            </button> -->
          </div>
        </div>
      </transition-group>
    </div>
  </div>
</template>
