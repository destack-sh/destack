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

function close() {
  destroyOverlayMenu();
  canvas.restoreComponentFocus();
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
    <!-- Classic context menu -->
    <Menu
      ref="menuRef"
      v-if="activeOverlayMenu?.info.kind == 'menu'"
      :key="activeOverlayMenu.id"
      class="pointer-events-auto absolute z-70"
      data-outside-view="true"
      v-bind="activeOverlayMenu.info"
      @close="close"
    />
    <!-- Generic component menu -->
    <div
      ref="menuRef"
      v-else-if="activeOverlayMenu?.info.kind == 'component'"
      class="pointer-events-auto absolute z-70 flex min-w-60 flex-col rounded-md border border-gray-400 bg-white py-1 text-gray-900 shadow-md shadow-gray-400"
      v-outside.mousedown.stop="close"
    >
      nocheckin
      {{ activeOverlayMenu?.info.props }}
    </div>
  </Transition>
</template>
