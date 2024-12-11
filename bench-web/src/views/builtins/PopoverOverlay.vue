<script lang="ts" setup>
import { canvas } from "@/system/space";
import { activePopovers, popPopover, topPopover, type PopoverInfo, type PopoverInstance } from "@/ui/popover";
import { focusInElement } from "@/ui/view";
import { getElement } from "@/utils/element";
import { getFloatingPosition, type FloatingPlacement } from "@/utils/floating";
import { log } from "@/utils/log";
import Menu from "@/views/builtins/Menu.vue";
import { getViewComponent } from "@/views/registry";
import { useElementSize, useEventListener, type MaybeElement } from "@vueuse/core";
import { computed, nextTick, ref, shallowRef, triggerRef, watch, type Ref } from "vue";

const popoverContainerRefs: Ref<Record<number, MaybeElement>> = shallowRef({});
const popoverInnerRefs: Ref<Record<number, MaybeElement>> = shallowRef({});
const popoverValues: Ref<Record<number, any>> = ref({});
const topPopoverContainer = computed(() => popoverContainerRefs.value[topPopover.value?.id]);
const topPopoverSize = useElementSize(topPopoverContainer, undefined, { box: "border-box" });
const shouldAnimate = computed(() => !activePopovers.value.some((popover) => popover.dontAnimate));

function registerContainerRef(popover: PopoverInstance, ref: any | undefined) {
  popover.element = ref;
  if (ref === popoverContainerRefs.value[popover.id]) return;
  else if (ref != null) popoverContainerRefs.value[popover.id] = ref;
  else delete popoverContainerRefs.value[popover.id];
  triggerRef(popoverContainerRefs);
}

function registerInnerRef(id: number, ref: any | undefined) {
  if (ref === popoverInnerRefs.value[id]) return;
  else if (ref != null) popoverInnerRefs.value[id] = ref;
  else delete popoverInnerRefs.value[id];
  triggerRef(popoverInnerRefs);
}

function getEnterFrom(placement: FloatingPlacement): string {
  if (placement.startsWith("left")) return "translate-x-[4px]";
  else if (placement.startsWith("top")) return "translate-y-[4px]";
  else if (placement.startsWith("right")) return "translate-x-[-4px]";
  /* bottom */ else return "translate-y-[-4px]";
}

// float position for topmost popover (the rest remains fixed)
watch([popoverContainerRefs, topPopoverSize.width, topPopoverSize.height], () => {
  const popoverRef = popoverContainerRefs.value[topPopover.value?.id];
  if (popoverRef == null || topPopover.value == null) return;

  // get bounding
  const popover = topPopover.value;
  const el = getElement(popoverRef)!;
  const referenceRect =
    popover.reference instanceof HTMLElement || popover.reference instanceof SVGElement
      ? popover.reference.getBoundingClientRect()
      : { x: popover.reference.x, y: popover.reference.y, width: 1, height: 1 };
  const containerRect =
    popover.container != null
      ? popover.container.getBoundingClientRect()
      : { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };

  // position
  const { x, y } = getFloatingPosition({
    floating: el.getBoundingClientRect(),
    reference: referenceRect,
    container: containerRect,
    options: popover,
  });
  el.style.position = "fixed";
  el.style.left = x + "px";
  el.style.top = y + "px";
});

// init popover value if set
watch(activePopovers, () => {
  activePopovers.value.forEach((popover) => {
    if (popover.kind != "menu" && popover.props.modelValue != null && popoverValues.value[popover.id] == null) {
      popoverValues.value[popover.id] = popover.props.modelValue;
    }
  });
});

// focus
function focus() {
  const focusTarget = popoverInnerRefs.value[topPopover.value?.id] ?? popoverContainerRefs.value[topPopover.value?.id];
  if (!focusInElement(focusTarget)) {
    log.warn("popover.focusFailed", topPopover.value?.kind, topPopover);
  }
}
watch(popoverContainerRefs, () => {
  if (topPopover.value != null && !topPopover.value?.dontFocus) {
    // auto-focus when created
    // NOTE: We must focus in the *next* tick even though we're already mounted.
    //  Chromium has a bug where it gets confused about the actual position of the containing elements (?)
    //    if we immediately focus it, breaking our floating positioning.
    nextTick(focus);
  }
});

// close popovers on click outside
useEventListener("mousedown", (e) => {
  if (activePopovers.value.length == 0) return; // nothing to do
  // find first component down the stack that contains the element (if any)
  let sliceFromIdx: number = -1;
  let generation: number | undefined;
  for (let i = activePopovers.value.length - 1; i >= 0; i--) {
    const popover = activePopovers.value[i];
    const containerEl = getElement(popoverContainerRefs.value[popover.id]);
    if (containerEl?.contains(e.target as HTMLElement)) {
      sliceFromIdx = i + 1;
      generation = popover.generation;
      break;
    }
  }
  popPopover(sliceFromIdx, generation);
  if (Object.values(activePopovers.value).length == 0) canvas.restoreComponentFocus();
});

function toComponent(info: PopoverInfo): any {
  if (info.kind != "view") throw new Error("not a component popover");
  else if (typeof info.component == "object") return info.component;
  else return getViewComponent(info.component);
}

function onApply(popover: PopoverInstance, value: any | undefined) {
  popover.onApply?.(value ?? popoverValues.value[popover.id]);
}

function close(popover: PopoverInstance | undefined) {
  const closedMenus = popPopover(popover);
  closedMenus?.forEach((popover) => popover.onClose?.());
  if (Object.values(activePopovers.value).length == 0) canvas.restoreComponentFocus();
}
</script>
<template>
  <TransitionGroup
    :enter-active-class="'transition-all ease-in ' + (shouldAnimate ? 'duration-75' : 'duration-0')"
    :enter-from-class="'opacity-0 ' + getEnterFrom(topPopover?.placement ?? 'top')"
    enter-to-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    :leave-active-class="'transition-all ease-out ' + (shouldAnimate ? 'duration-75' : 'duration-0')"
    leave-from-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    :leave-to-class="'opacity-0 ' + getEnterFrom(topPopover?.placement ?? 'top')"
  >
    <template v-for="popover in activePopovers" :key="popover.id">
      <!-- Menu -->
      <Menu
        v-if="popover?.kind == 'menu'"
        :ref="(el) => registerContainerRef(popover, el)"
        :key="popover.id"
        class="pointer-events-auto absolute z-70 shadow-sm shadow-gray-300"
        :style="{
          width: popover.width != null ? popover.width + 'px' : '',
          height: popover.height != null ? popover.height + 'px' : '',
        }"
        data-outside-view="true"
        v-bind="popover"
        @close="() => close(popover)"
      />
      <!-- View -->
      <div
        v-else-if="popover?.kind == 'view'"
        :ref="(el) => registerContainerRef(popover, el)"
        class="pointer-events-auto absolute z-70 flex flex-col rounded border border-gray-200 bg-white text-gray-900 shadow-sm shadow-gray-300"
        :style="{
          width: popover.props?.size?.width != null ? popover.props.size.width + 'px' : '',
          height: popover.props?.size?.height != null ? popover.props.size.height + 'px' : '',
        }"
        :class="popover.containerClass"
        data-outside-view="true"
        @keydown.esc.stop.prevent="() => close(popover)"
      >
        <component
          :is="toComponent(popover)"
          id="popover"
          :ref="(el: any) => registerInnerRef(popover.id, el)"
          v-bind="{ isInline: true, isPopover: true, ...(popover.props ?? {}) }"
          :model-value="popoverValues[popover.id]"
          @update:model-value="
            (newValue: any) => {
              popoverValues[popover.id] = newValue;
              popover.onUpdate?.(newValue);
            }
          "
          @apply="(value: any, keepOpen?: boolean) => (onApply(popover, value), keepOpen || close(popover))"
          @close="() => close(popover)"
        />
      </div>
    </template>
  </TransitionGroup>
</template>
