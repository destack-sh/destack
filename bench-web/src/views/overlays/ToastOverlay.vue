<script lang="ts" setup>
import { ColorShade, ColorType, IconData } from "@/proto/wire";
import { IconInline, makeIcon } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { Toast, toaster, ToastLevel, type ToastAnchor } from "@/ui/toast";
import { assertNever } from "@/utils/functools";
import { computed } from "vue";

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

const COLOR_BY_TOAST_LEVEL: Record<ToastLevel, ColorType> = {
  [ToastLevel.DEBUG]: ColorType.GRAY,
  [ToastLevel.INFO]: ColorType.YELLOW,
  [ToastLevel.SUCCESS]: ColorType.SUCCESS,
  [ToastLevel.WARNING]: ColorType.WARNING,
  [ToastLevel.ERROR]: ColorType.DANGER,
};
const ICON_BY_TOAST_LEVEL: Record<ToastLevel, IconData> = {
  [ToastLevel.DEBUG]: makeIcon("fas fa-bug"),
  [ToastLevel.INFO]: makeIcon("fas fa-info-circle"),
  [ToastLevel.SUCCESS]: makeIcon("fas fa-check-circle"),
  [ToastLevel.WARNING]: makeIcon("fas fa-exclamation-triangle"),
  [ToastLevel.ERROR]: makeIcon("fas fa-exclamation-triangle"),
};

function getToastColorHex(toast: Toast, shade: ColorShade): string {
  const color = COLOR_BY_TOAST_LEVEL[toast.level];
  return getColorHex(color, shade)!;
}

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
    return assertNever(props.anchor);
  }
});
</script>
<template>
  <TransitionGroup
    tag="ul"
    class="fixed z-50 flex gap-y-1.5 p-2"
    data-outside-view="true"
    :class="[isInverted ? 'flex-col-reverse' : 'flex-col']"
    :style="absoluteStyle"
    move-class="transition-all duration-100"
    enter-active-class="transition-all ease-in duration-150"
    :enter-from-class="'scale-90 transform ' + ENTER_FROM_BY_ANCHOR[props.anchor]"
    enter-to-class="scale-100 transform translate-x-0 translate-y-0"
    leave-active-class="transition-all ease-out duration-200"
    leave-from-class="scale-100 transform translate-x-0 translate-y-0"
    :leave-to-class="'scale-100 transform ' + LEAVE_TO_BY_ANCHOR[props.anchor]"
  >
    <!-- Toasts -->
    <li
      v-for="(toast, i) in visibleToasts"
      :key="toast.id"
      :style="{
        width: TOAST_WIDTH + 'px',
      }"
      class="group/toast relative flex flex-row items-center gap-x-1 rounded-2xl border border-gray-300 bg-white shadow-xs"
    >
      <!-- Icon -->
      <div class="shrink-0 pl-3.5 pr-1.5">
        <IconInline
          v-bind="toast.icon ?? ICON_BY_TOAST_LEVEL[toast.level]"
          class="w-5 text-center text-lg text-gray-700"
          :style="{
            color: getToastColorHex(toast, ColorShade.S600),
          }"
        />
      </div>
      <!-- Main -->
      <div class="max-w-full min-w-0 py-1.5">
        <!-- Header -->
        <div class="flex flex-row max-w-full">
          <span class="font-medium text-gray-700 truncate">{{ toast.title }}</span>
        </div>
        <!-- Content -->
        <p v-if="toast.text" class="max-w-full truncate text-gray-400">
          {{ toast.text }}
        </p>
      </div>
      <!-- Dismiss -->
      <button
        class="right-3 ml-auto h-full shrink-0 border-gray-300 py-3 pl-1 pr-3.5 text-gray-400 opacity-40 transition-colors duration-75 group-hover/toast:opacity-100"
        @click="toaster.dismiss(toast)"
      >
        <i class="fas fa-xmark" />
      </button>
    </li>
  </TransitionGroup>
</template>
