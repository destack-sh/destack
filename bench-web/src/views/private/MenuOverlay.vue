<script lang="tsx" setup>
import { canvas } from "@/system/space";
import { getElement } from "@/utils/element";
import { getFloatingPosition, type FloatingPlacement } from "@/utils/floating";
import { activeOverlayMenu, destroyOverlayMenu } from "@/utils/menu";
import Menu from "@/views/private/Menu.vue";
import type { MaybeElement } from "@vueuse/core";
import { ref, watch, type Ref } from "vue";

const menuRef: Ref<InstanceType<typeof Menu> | null> = ref(null);

function getEnterFrom(placement: FloatingPlacement): string {
  if (placement.startsWith("left")) return "translate-x-[4px]";
  else if (placement.startsWith("top")) return "translate-y-[4px]";
  else if (placement.startsWith("right")) return "translate-x-[-4px]";
  /* bottom */ else return "translate-y-[-4px]";
}

watch([menuRef, activeOverlayMenu], () => {
  if (menuRef.value == null || activeOverlayMenu.value == null) return;
  // get bounding
  const menu = activeOverlayMenu.value;
  const el = getElement(menuRef.value as MaybeElement)!;
  const referenceRect = { x: menu.reference.x, y: menu.reference.y, width: 1, height: 1 };
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
    <!-- Classic context menu -->
    <Menu
      ref="menuRef"
      v-if="activeOverlayMenu"
      :key="activeOverlayMenu.id"
      class="pointer-events-auto absolute z-70"
      data-outside-view="true"
      v-bind="activeOverlayMenu.info"
      @close="() => (destroyOverlayMenu(), $nextTick(() => canvas.restoreComponentFocus()))"
    />
  </Transition>
</template>
