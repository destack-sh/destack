<script lang="ts" setup>
import { canvas } from "@/system/space";
import { focusInElement } from "@/ui/canvas";
import {
  activePopovers,
  popPopover,
  topPopover,
  updatePopover,
  type PopoverInfo,
  type PopoverInstance,
} from "@/ui/popover";
import { getElement } from "@/utils/element";
import { getFloatingPosition, type FloatingPlacement } from "@/utils/floating";
import { log } from "@/utils/log";
import Menu from "@/views/builtins/Menu.vue";
import { getViewComponent } from "@/views/registry";
import { useElementSize, useEventListener, type MaybeElement } from "@vueuse/core";
import { computed, nextTick, shallowRef, toValue, triggerRef, watch, watchEffect, type Ref } from "vue";

const popoverContainerRefs: Ref<Record<number, MaybeElement>> = shallowRef({});
const popoverInnerRefs: Ref<Record<number, MaybeElement>> = shallowRef({});
const popoverValues: Ref<Record<number, any>> = shallowRef({});
const topPopoverContainer = computed(() => popoverContainerRefs.value[topPopover.value?.id]);
const topPopoverSize = useElementSize(topPopoverContainer, undefined, { box: "border-box" });

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

// auto update computed props values
watchEffect(() => {
  activePopovers.value.forEach((popover) => {
    if (popover.info.kind == "component" && popover.info.propsRef != null) {
      popover.info.props = { ...popover.info.props, ...toValue(popover.info.propsRef) };
      popoverValues.value[popover.id] = popover.info.props.modelValue;
      triggerRef(popoverValues);
    }
  });
});

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
    options: popover.info,
  });
  el.style.position = "fixed";
  el.style.left = x + "px";
  el.style.top = y + "px";
});

// init popover value if set
watch(activePopovers, () => {
  activePopovers.value.forEach((popover) => {
    if (
      popover.info.kind != "menu" &&
      popover.info.props.modelValue != null &&
      popoverValues.value[popover.id] == null
    ) {
      popoverValues.value[popover.id] = popover.info.props.modelValue;
    }
  });
});

// focus
function focus() {
  const focusTarget = popoverInnerRefs.value[topPopover.value?.id] ?? popoverContainerRefs.value[topPopover.value?.id];
  if (!focusInElement(focusTarget)) {
    log.warn("popover.focusFailed", topPopover.value?.info.kind, topPopover);
  }
}
watch(popoverContainerRefs, () => {
  if (topPopover.value != null && !topPopover.value?.info.dontFocus) {
    // auto-focus when created
    // NOTE: We must focus in the *next* tick even though we're already mounted.
    //  Chromium has a bug where it gets confused about the actual position of the containing elements (?)
    //    if we immediately focus it, breaking our floating positioning.
    nextTick(focus);
  }
});

// close popovers on click outside
useEventListener("mousedown", (e) => {
  // find first component down the stack that contains the element (if any)
  let sliceFromIdx: number = -1;
  for (let i = activePopovers.value.length - 1; i >= 0; i--) {
    const popover = activePopovers.value[i];
    const containerEl = getElement(popoverContainerRefs.value[popover.id]);
    if (containerEl?.contains(e.target as HTMLElement)) {
      sliceFromIdx = i + 1;
      break;
    }
  }
  popPopover(sliceFromIdx);
});

function toComponent(info: PopoverInfo): any {
  if (info.kind != "component") throw new Error("not a component popover");
  else if (typeof info.component == "object") return info.component;
  else return getViewComponent(info.component);
}

function fire(popover: PopoverInstance) {
  popover.info.onApply?.(popoverValues.value[popover.id]);
}

function close(popover: PopoverInstance | undefined) {
  const popoverIdx = activePopovers.value.findIndex((m) => m === popover);
  const closedMenus = popPopover(popoverIdx);
  closedMenus?.forEach((popover) => popover.info.onClose?.());
  if (Object.values(activePopovers.value).length == 0) canvas.restoreComponentFocus();
}
</script>
<template>
  <!-- NOTE :Robustness: sometime on hot reload PopoverOverlay recursive updates itself? -->
  <TransitionGroup
    enter-active-class="transition-all ease-in duration-75"
    :enter-from-class="'opacity-0 ' + getEnterFrom(topPopover?.info?.placement ?? 'top')"
    enter-to-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    :leave-to-class="'opacity-0 ' + getEnterFrom(topPopover?.info?.placement ?? 'top')"
  >
    <template v-for="popover in activePopovers" :key="popover.id">
      <!-- Context menu popover -->
      <Menu
        v-if="popover?.info.kind == 'menu'"
        :ref="(el) => registerContainerRef(popover, el)"
        :key="popover.id"
        class="pointer-events-auto absolute z-70"
        data-outside-view="true"
        v-bind="popover.info"
        @close="() => close(popover)"
      />
      <!-- Generic component popover -->
      <div
        v-else-if="popover?.info.kind == 'component'"
        :ref="(el) => registerContainerRef(popover, el)"
        class="pointer-events-auto absolute z-70 flex flex-col rounded border border-gray-300 bg-white text-gray-900"
        :style="{
          width: popover.info.props?.size?.width != null ? popover.info.props.size.width + 'px' : '',
          height: popover.info.props?.size?.height != null ? popover.info.props.size.height + 'px' : '',
        }"
        :class="popover.info.containerClass"
        data-outside-view="true"
        @keydown.esc.stop.prevent="() => close(popover)"
      >
        <component
          :is="toComponent(popover.info)"
          :ref="(el: any) => registerInnerRef(popover.id, el)"
          v-bind="{ isInline: true, ...(popover.info.props ?? {}) }"
          :model-value="popoverValues[popover.id]"
          @update:model-value="
            (newValue: any) => {
              popoverValues[popover.id] = newValue;
              popover.info.onUpdate?.(newValue);
            }
          "
          @update:self="
            (newProps: any) => {
              if (!('props' in popover.info)) throw new Error(`${popover.info.kind} popover has no props`);
              updatePopover(popover, { props: { ...popover.info.props, ...newProps } });
            }
          "
          @apply="() => (fire(popover), close(popover))"
          @close="() => close(popover)"
        />
      </div>
    </template>
  </TransitionGroup>
</template>
