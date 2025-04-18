<script lang="ts" setup>
import { clearLightbox, lightbox, LightboxInfo } from "@/ui/popover";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { getViewComponent } from "@/views/registry";
import { useElementSize, useEventListener, useMagicKeys } from "@vueuse/core";
import { ref, Ref } from "vue";

const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

function toComponent(info: LightboxInfo): any {
  if (typeof info.component == "object") {
    return info.component;
  } else {
    return getViewComponent(info.component);
  }
}

// auto-close on escape
useEventListener(document, "keydown", (e) => {
  if (e.key == "Escape") {
    clearLightbox();
  }
});

const headerRef: Ref<HTMLElement | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const headerSize = useElementSize(headerRef);
const bodySize = useElementSize(bodyRef);
</script>
<template>
  <!-- Backdrop -->
  <Transition
    enter-active-class="transition-opacity ease-in duration-75"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 translate-y-0"
    leave-to-class="opacity-0 translate-y-[-10px]"
    appear
  >
    <div
      v-if="lightbox != null"
      class="fixed left-0 top-0 z-80 flex h-screen w-screen flex-col items-center justify-center bg-gray-700/80 px-12 backdrop-blur-xs"
      data-outside-view="true"
      @click.stop.prevent="() => clearLightbox()"
      @keydown.esc="() => clearLightbox()"
    >
      <!-- Header -->
      <div ref="headerRef" class="fixed top-10 z-70 my-1 flex w-full flex-row justify-center px-12">
        <!-- Info (left) -->
        <div class="rounded-sm px-3 py-1">
          <NodeReference class="text-white" :node="lightbox.node" is-light is-minimal is-icon-light size="title" />
        </div>
      </div>
      <!-- Body -->
      <Transition
        enter-active-class="transition-all ease-in duration-150"
        enter-from-class="scale-[0.98]"
        enter-to-class="scale-100"
        appear
      >
        <div v-if="lightbox != null" ref="bodyRef" class="pointer-events-auto z-60 rounded-2xl">
          <component
            :is="toComponent(lightbox)"
            id="lightbox"
            ref="lightboxRef"
            v-bind="{
              ...(lightbox.props ?? {}),
              isInline: true,
              isPopover: true,
              isLightbox: true,
              size: { width: box.width, height: box.height - 200 },
            }"
            @close="() => clearLightbox()"
          />
        </div>
      </Transition>
    </div>
  </Transition>
</template>
