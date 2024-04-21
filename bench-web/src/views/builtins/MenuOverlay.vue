<script lang="ts" setup>
import { canvas } from "@/system/space";
import { focusInElement } from "@/views/canvas";
import { getElement } from "@/utils/element";
import { getFloatingPosition, type FloatingPlacement } from "@/utils/floating";
import { activeOverlayMenu, destroyOverlayMenu } from "@/utils/menu";
import Menu from "@/views/builtins/Menu.vue";
import { useElementSize, whenever, type MaybeElement } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const menuRefContainer: Ref<MaybeElement> = ref(null);
const menuRefInner: Ref<MaybeElement> = ref(null);
const menuRef = computed(() => menuRefInner.value ?? menuRefContainer.value);
const menuContainerSize = useElementSize(menuRefContainer);
const menuRefValue: Ref<any> = ref(null);

function getEnterFrom(placement: FloatingPlacement): string {
  if (placement.startsWith("left")) return "translate-x-[4px]";
  else if (placement.startsWith("top")) return "translate-y-[4px]";
  else if (placement.startsWith("right")) return "translate-x-[-4px]";
  /* bottom */ else return "translate-y-[-4px]";
}

// float position
watch([menuRef, menuContainerSize.width, menuContainerSize.height, activeOverlayMenu], () => {
  if (menuRef.value == null || activeOverlayMenu.value == null) return;

  // get bounding
  const menu = activeOverlayMenu.value;
  const el = getElement(menuRefContainer.value as MaybeElement)!;
  const referenceRect =
    menu.reference instanceof HTMLElement || menu.reference instanceof SVGElement
      ? menu.reference.getBoundingClientRect()
      : { x: menu.reference.x, y: menu.reference.y, width: 1, height: 1 };
  const containerRect =
    menu.container != null
      ? menu.container.getBoundingClientRect()
      : { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };

  // position
  const { x, y } = getFloatingPosition({
    floating: el.getBoundingClientRect(),
    reference: referenceRect,
    container: containerRect,
    options: menu.info,
  });
  el.style.position = "fixed";
  el.style.left = x + "px";
  el.style.top = y + "px";
});

// init menuRefValue if set
whenever(activeOverlayMenu, () => {
  if (activeOverlayMenu.value?.info.kind == "component") {
    menuRefValue.value = (activeOverlayMenu.value.info.props as any)?.modelValue ?? null;
  }
});

// focus
function focus() {
  if (!focusInElement(menuRef.value)) throw new Error(`failed to focus in ${activeOverlayMenu.value?.info.kind}`);
}
whenever(menuRef, () => {
  if (!activeOverlayMenu.value?.info.dontFocus) {
    // auto-focus when created
    // NOTE: We must focus in the *next* tick even though we're already mounted.
    //  Chromium has a bug where it gets confused about the actual position of the containing elements (?)
    //    if we immediately focus it, breaking our floating positioning.
    nextTick(focus);
  }
});

function fire() {
  activeOverlayMenu.value?.info.onApply?.(menuRefValue.value);
}

function close() {
  if (!activeOverlayMenu.value) return;
  activeOverlayMenu.value?.info.onClose?.();
  destroyOverlayMenu();
  if (!activeOverlayMenu.value?.info.dontFocus) canvas.restoreComponentFocus();
}
</script>
<template>
  <Transition
    enter-active-class="transition-all ease-in duration-75"
    :enter-from-class="'opacity-0 ' + getEnterFrom(activeOverlayMenu?.info.placement ?? 'top')"
    enter-to-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    :leave-to-class="'opacity-0 ' + getEnterFrom(activeOverlayMenu?.info.placement ?? 'top')"
    mode="out-in"
  >
    <!-- Classic menu -->
    <Menu
      ref="menuRefContainer"
      v-if="activeOverlayMenu?.info.kind == 'menu'"
      :key="activeOverlayMenu.id"
      class="pointer-events-auto absolute z-70"
      data-outside-view="true"
      v-bind="activeOverlayMenu.info"
      @close="close"
    />
    <!-- Generic component menu -->
    <div
      ref="menuRefContainer"
      v-else-if="activeOverlayMenu?.info.kind == 'component'"
      class="pointer-events-auto absolute z-70 flex min-w-60 flex-col rounded border border-gray-400 bg-white text-gray-900"
      :class="activeOverlayMenu.info.containerClass"
      v-outside.mousedown.stop="close"
      @keydown.enter.stop.prevent="fire(), close()"
      @keydown.esc.stop.prevent="close"
    >
      <component
        ref="menuRefInner"
        :is="activeOverlayMenu.info.component"
        v-bind="(activeOverlayMenu.info.props ?? {})"
        v-model="menuRefValue"
        @apply="fire(), close()"
      />
    </div>
  </Transition>
</template>
