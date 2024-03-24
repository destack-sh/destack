<script lang="ts" setup>
import { Orientation } from "@/proto/wire";
import { useActiveConnection } from "@/system/connection";
import { LOCAL_SPACE_PTR, spacePtr } from "@/system/local";
import { space } from "@/system/space";
import { isDragging } from "@/utils/layout";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/private/Bar.vue";
import Omnibar from "@/views/private/Omnibar.vue";
import ToastOverlay from "@/views/private/ToastOverlay.vue";
import TooltipOverlay from "@/views/private/TooltipOverlay.vue";
import MenuOverlay from "@/views/private/MenuOverlay.vue";
import { useWindowSize } from "@vueuse/core";
import { computed, ref } from "vue";

const BAR_HEIGHT = 42;
const BAR_OFFSET = 0;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const windowRef = ref<InstanceType<typeof Windowed> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const { graph: spaceGraph, connection: spaceConnection } = useActiveConnection(
  computed(() => spacePtr.value ?? LOCAL_SPACE_PTR),
);

const mainBox = computed(() => ({
  left: 0,
  top: BAR_HEIGHT,
  width: spaceWidth.value,
  height: spaceHeight.value - BAR_HEIGHT - BAR_OFFSET,
}));
const omnibarRef = ref<InstanceType<typeof Omnibar> | null>(null);
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="scrollbar-none max-h-screen w-full overflow-hidden overscroll-none bg-gray-100 text-sm"
    :class="[isDragging ? 'yselect-none pointer-events-none' : '']"
    @contextmenu.stop.prevent="() => {} /* suppress generic context menu */"
  >
    <!-- Bar -->
    <Bar
      ref="barRef"
      class="w-full border-b-2 border-gray-300"
      :style="{ height: BAR_HEIGHT + 'px' }"
      :space-graph="spaceGraph"
      :space-connection="spaceConnection"
      :box="{ x: 0, y: 0, width: spaceWidth, height: BAR_HEIGHT }"
    />
    <!-- Window root -->
    <Windowed
      ref="windowRef"
      v-if="spacePtr && space"
      :self="spacePtr"
      :size="mainBox"
      :focus="space.focus"
      :name="space.name"
      :title="space.name"
      :text="space.text"
      :orientation="Orientation.HORIZONTAL"
      :style="{ marginTop: BAR_OFFSET + 'px' }"
    />
    <!-- Overlays -->
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <TooltipOverlay />
    <MenuOverlay />
  </div>
</template>
<style>
/* stop overscrolling */
* {
  overscroll-behavior: none;
  scrollbar-gutter: overlay;
}

/* hide scrollbar with .scrollbar-none */
.scrollbar-none {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.scrollbar-none::-webkit-scrollbar {
  display: none;
}

/** make selections match primary color */
::selection {
  background-color: #fcd34d;
}

.caret-transparent {
  caret-color: transparent;
}

/* marks shouldn't be ugly */
mark {
  background-color: transparent;
  color: inherit;
  font-weight: bold;
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>
