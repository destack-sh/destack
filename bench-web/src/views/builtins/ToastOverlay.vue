<script lang="tsx" setup>
import { IconInline } from "@/system/icon";
import { toaster, type ToastAnchor } from "@/system/toast";
import { computed } from "vue";
import { BG_COLOR_BY_LEVEL, ACCENT_COLOR_BY_LEVEL, DEFAULT_ICON_BY_LEVEL } from "@/utils/style";

const props = defineProps<{ anchor: ToastAnchor; box: { left: number; top: number; width: number; height: number } }>();
// default order is most recent bottom
const isInverted = computed(() => props.anchor == "top-left" || props.anchor == "top-right");

// enter from top/bottom
const TOAST_WIDTH = 320;
const MAX_TOASTS = 5;

const ENTER_FROM_BY_ANCHOR: Record<ToastAnchor, string> = {
  "top-left": "translate-y-[-100%]",
  "top-right": "translate-y-[-100%]",
  "bottom-left": "translate-y-[100%]",
  "bottom-right": "translate-y-[100%]",
};
// leave to left/right
const LEAVE_TO_BY_ANCHOR: Record<ToastAnchor, string> = {
  "top-left": "translate-x-[-320px]",
  "top-right": "translate-x-[320px]",
  "bottom-left": "translate-x-[-320px]",
  "bottom-right": "translate-x-[320px]",
};

const visibleToasts = computed(() => {
  let toasts = toaster.activeToasts;
  if (toasts.length > MAX_TOASTS) toasts = toasts.slice(toasts.length - MAX_TOASTS);
  return toasts;
});

const absoluteStyle = computed(() => {
  if (props.anchor == "top-left") {
    return { left: props.box.left + "px", top: props.box.top + "px" };
  } else if (props.anchor == "top-right") {
    return { right: "0px", top: props.box.top + "px" };
  } else if (props.anchor == "bottom-left") {
    return { left: props.box.left + "px", bottom: "0px" };
  } else if (props.anchor == "bottom-right") {
    return { right: "0px", bottom: "0px" };
  } else {
    throw new Error("unexpected anchor: " + props.anchor);
  }
});
</script>
<template>
  <TransitionGroup
    tag="ul"
    class="fixed z-50 flex gap-y-2 p-2"
    data-outside-view="true"
    :class="[isInverted ? 'flex-col-reverse' : 'flex-col']"
    :style="absoluteStyle"
    move-class="transition-all duration-100"
    enter-active-class="transition-all ease-in duration-150"
    :enter-from-class="'scale-95 transform ' + ENTER_FROM_BY_ANCHOR[props.anchor]"
    enter-to-class="scale-100 transform translate-x-0 translate-y-0"
    leave-active-class="transition-all ease-out duration-200"
    leave-from-class="scale-100 transform translate-x-0 translate-y-0"
    :leave-to-class="'scale-95 transform ' + LEAVE_TO_BY_ANCHOR[props.anchor]"
  >
    <!-- Toasts -->
    <li
      v-for="toast in visibleToasts"
      :key="toast.id"
      :style="{ width: TOAST_WIDTH + 'px' }"
      class="group relative rounded-md border border-gray-300 bg-white px-4 py-3 shadow-md shadow-gray-300"
    >
      <!-- Level indicator -->
      <div
        class="absolute left-0 top-0 h-1 w-full rounded-md transition-transform duration-200"
        :class="BG_COLOR_BY_LEVEL[toast.level]"
      />
      <!-- Body -->
      <div class="flex flex-row">
        <!-- Icon -->
        <div class="w-4 text-center">
          <IconInline
            v-bind="toast.icon ?? DEFAULT_ICON_BY_LEVEL[toast.level]"
            class="mt-0.5"
            :class="[ACCENT_COLOR_BY_LEVEL[toast.level]]"
          />
        </div>
        <div class="ml-2.5">
          <!-- Title -->
          <span class="font-medium text-gray-900">{{ toast.title }}</span>
          <!-- Content -->
          <p v-if="toast.text" class="mt-0.5 text-gray-500">{{ toast.text }}</p>
        </div>
      </div>
      <!-- Actions -->
      <div v-if="toast.actions.length > 0" class="mt-1.5 flex w-full justify-end gap-x-3">
        <button
          v-for="(action, i) in toast.actions"
          :key="i"
          class="max-w-20 truncate font-medium"
          :class="[action.isPrimary ? 'text-gray-700 hover:text-primary-900' : 'text-gray-500 hover:text-primary-900']"
          @click="action.action(), toaster.dismiss(toast)"
        >
          <IconInline v-if="action.icon" v-bind="action.icon" class="mr-1" />
          <span>{{ action.title }}</span>
        </button>
      </div>
      <!-- Dismiss -->
      <button class="absolute right-3 top-2.5 text-gray-300 group-hover:text-gray-400" @click="toaster.dismiss(toast)">
        <i class="fas fa-xmark hover:text-primary-900" />
      </button>
    </li>
  </TransitionGroup>
</template>
