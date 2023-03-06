<script lang="ts" setup>
import { useNotifications, type DisplayNotification } from "@/state/notifications";
import { CheckCircleIcon, ExclamationCircleIcon, InformationCircleIcon, XCircleIcon } from "@heroicons/vue/24/outline";

const notifications = useNotifications();

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
        <div
          v-for="notification in notifications.shownNotifications.value"
          :key="notification.localId"
          class="pointer-events-auto flex w-full max-w-sm items-center overflow-hidden rounded-sm bg-white p-3 shadow-md ring-1 ring-orange-900 ring-opacity-40"
          @mouseenter="freezeNotification(notification)"
        >
          <!-- Message body-->
          <div class="flex flex-1 flex-row justify-between">
            <!-- Icon -->
            <div class="-mt-[1px]">
              <component
                :is="getIcon(notification)"
                class="h-5 w-5"
                :class="{
                  'text-orange-600': notification.kind === 'notice' || notification.kind === 'success',
                  'text-red-500': notification.kind === 'error',
                  'text-yellow-500': notification.kind === 'warning',
                }"
              />
            </div>
            <!-- Main message -->
            <div class="ml-3 flex flex-1 flex-col">
              <h3 class="text-sm font-bold text-gray-900">{{ notification.message }}</h3>
              <p v-if="notification.description" class="pt-1 text-xs text-gray-500">{{ notification.description }}</p>
            </div>
            <!-- Actions -->
            <button
              v-if="notification.actionText"
              type="button"
              class="mx-3 h-fit flex-shrink-0 self-center rounded-sm px-3 py-1 text-sm font-medium underline decoration-gray-500 decoration-dashed underline-offset-4 hover:decoration-gray-900 hover:decoration-solid focus:outline-none"
              @click="() => (notification.action?.(), notifications.dismiss(notification.localId))"
            >
              {{ notification.actionText }}
            </button>
          </div>
          <!-- Dismiss -->
          <!-- <div class="flex flex-shrink-0 ml-4">
            <button
              type="button"
              @click="notifications.dismiss(notification.id)"
              class="inline-flex text-gray-200 rounded-md hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            >
              <span class="sr-only">Close</span>
              <XMarkIcon class="w-4 h-4" aria-hidden="true" />
            </button>
          </div> -->
        </div>
      </transition-group>
    </div>
  </div>
</template>
