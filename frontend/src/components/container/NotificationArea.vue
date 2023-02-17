<script lang="ts" setup>
import { useNotifications } from "@/state/notifications";
import { CheckCircleIcon, ExclamationCircleIcon, InformationCircleIcon, XCircleIcon } from "@heroicons/vue/20/solid";
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
          class="pointer-events-auto flex w-full max-w-sm items-center overflow-hidden rounded-sm border-l-2 bg-white p-3 shadow-md ring-1 ring-black ring-opacity-5"
          :class="{
            'border-l-white': notification.kind === 'notice',
            'border-red-500': notification.kind === 'error',
            'border-yellow-500': notification.kind === 'warning',
            'border-green-500': notification.kind === 'success',
          }"
        >
          <!-- Message body-->
          <div class="flex flex-1 justify-between">
            <!-- Icon -->
            <div class="-mt-[1px]">
              <component
                :is="
                  {
                    error: XCircleIcon,
                    notice: InformationCircleIcon,
                    warning: ExclamationCircleIcon,
                    success: CheckCircleIcon,
                  }[notification.kind]
                "
                class="h-5 w-5"
                :class="{
                  'text-orange-600': notification.kind === 'notice',
                  'text-red-500': notification.kind === 'error',
                  'text-yellow-500': notification.kind === 'warning',
                  'text-green-500': notification.kind === 'success',
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
              class="mx-3 h-fit flex-shrink-0 self-center rounded-sm bg-orange-600 py-1 px-3 text-sm font-medium text-white focus:outline-none"
              @click="() => (notification.action?.(), notifications.dismiss(notification.id))"
            >
              {{ notification.actionText }}
            </button>
          </div>
          <!-- Dismiss -->
          <!-- <div class="ml-4 flex flex-shrink-0">
            <button
              type="button"
              @click="notifications.dismiss(notification.id)"
              class="inline-flex rounded-md text-gray-200 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            >
              <span class="sr-only">Close</span>
              <XMarkIcon class="h-4 w-4" aria-hidden="true" />
            </button>
          </div> -->
        </div>
      </transition-group>
    </div>
  </div>
</template>
