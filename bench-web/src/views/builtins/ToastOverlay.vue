<script lang="ts" setup>
import { ColorShade, ColorType } from "@/proto/wire";
import { IconInline } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { Toast, toaster, ToastLevel, type ToastAnchor } from "@/ui/toast";
import { assertNever } from "@/utils/functools";
import { computed } from "vue";

const props = defineProps<{ anchor: ToastAnchor; box: { left: number; top: number; width: number; height: number } }>();
// default order is most recent bottom
const isInverted = computed(() => props.anchor == "top-left" || props.anchor == "top-right");

// enter from top/bottom
const TOAST_WIDTH = 360;
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
  [ToastLevel.INFO]: ColorType.GRAY,
  [ToastLevel.SUCCESS]: ColorType.SUCCESS,
  [ToastLevel.WARNING]: ColorType.WARNING,
  [ToastLevel.ERROR]: ColorType.DANGER,
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
      v-for="(toast, i) in visibleToasts"
      :key="toast.id"
      :style="{
        width: TOAST_WIDTH + 'px',
        borderLeftColor: getToastColorHex(toast, ColorShade.S600),
      }"
      class="group/toast relative rounded border-l-4 bg-white px-3.5 py-2"
    >
      <!-- Header -->
      <div class="flex flex-row">
        <span class="font-medium text-gray-900">{{ toast.title }}</span>
      </div>
      <!-- Content -->
      <p v-if="toast.text" class="mt-0.5 line-clamp-2 text-gray-700">
        {{ toast.text }}
      </p>
      <!-- Actions -->
      <div v-if="toast.actions.length > 0" class="mt-1.5 flex w-full justify-end gap-x-3">
        <button
          v-for="(action, j) in toast.actions"
          :key="j"
          class="group/action max-w-20 truncate font-medium text-gray-400 group-hover/action:text-gray-700"
          @click="action.action(), toaster.dismiss(toast)"
        >
          <IconInline v-if="action.icon" v-bind="action.icon" class="mr-1" />
          <span class="">{{ action.title }}</span>
        </button>
      </div>
      <!-- Dismiss -->
      <button
        class="absolute right-3 top-2.5 text-gray-400 opacity-40 transition-colors duration-75 hover:bg-gray-100 group-hover/toast:opacity-100"
        @click="toaster.dismiss(toast)"
      >
        <i class="fas fa-xmark" />
      </button>
    </li>
  </TransitionGroup>
</template>
