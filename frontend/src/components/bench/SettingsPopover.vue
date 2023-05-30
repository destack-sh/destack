<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import {
  CONTENT_MARGIN_X_NARROW,
  CONTENT_MARGIN_X_WIDE,
  CONTENT_WIDTH_NARROW,
  CONTENT_WIDTH_WIDE,
  useAppearance,
} from "@/state/appearance";
import { useBenchState } from "@/state/editor";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { ArrowsPointingOutIcon, Bars3BottomLeftIcon, HashtagIcon, MapIcon, MoonIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const appearance = useAppearance();
const bench = useBenchState();

const isContentWide = computed(() => appearance.contentWidth != CONTENT_WIDTH_NARROW);
function setContentWide(wide: boolean) {
  appearance.contentWidth = wide ? CONTENT_WIDTH_WIDE : CONTENT_WIDTH_NARROW;
  appearance.contentMarginX = wide ? CONTENT_MARGIN_X_WIDE : CONTENT_MARGIN_X_NARROW;
}

const fontOptions = [
  {
    key: "sans",
    label: "Sans",
    style: "font-sans",
    default: true,
  },
  {
    key: "serif",
    label: "Serif",
    style: "font-serif",
    default: false,
  },
  {
    key: "mono",
    label: "Mono",
    style: "font-mono",
    default: false,
  },
];
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute bottom-1 left-14 z-40 flex w-60 flex-col gap-2 rounded-sm bg-white px-2 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Font style -->
        <div class="w-full px-2 pb-1">
          <!-- <span class="text-xs text-gray-500">Style</span> -->
          <div class="flex flex-row justify-between">
            <button
              v-for="font in fontOptions"
              :key="font.key"
              class="flex flex-col items-center justify-center rounded-sm px-5 text-center hover:bg-orange-100"
              :class="font.style"
              @click="appearance.font = font.key"
            >
              <span class="text-2xl" :class="{ 'text-orange-600': appearance.font == font.key }">Ag</span>
              <span class="font-sans text-xs text-gray-500">{{ font.label }}</span>
            </button>
          </div>
        </div>
        <!-- Font size -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <Bars3BottomLeftIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Big text</span>
          </span>
          <Switch
            :model-value="!appearance.textSmall"
            @update:model-value="appearance.textSmall = !appearance.textSmall"
          />
        </div>
        <!-- Content width -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <ArrowsPointingOutIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Wide content</span>
          </span>
          <Switch :model-value="isContentWide" @update:model-value="setContentWide" />
        </div>
        <!-- Line numbers -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <HashtagIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Line numbers</span>
          </span>
          <Switch v-model="bench.showLineNumbers" />
        </div>
        <!-- Editor headers -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MapIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Global header</span>
          </span>
          <Switch v-model="bench.showGlobalHeader" />
        </div>
        <!-- Editor headers -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MapIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Editor tabs</span>
          </span>
          <Switch v-model="bench.showEditorGroupHeader" />
        </div>
        <!-- Dark mode -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MoonIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Dark mode (soon)</span>
          </span>
          <Switch
            :model-value="appearance.theme != 'light'"
            @update:model-value="appearance.theme = $event ? 'dark' : 'light'"
          />
        </div>
        <!-- Zen mode -->
        <!-- (should become an action) -->
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
