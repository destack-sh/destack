<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { useAppearance, type Font } from "@/state/appearance";
import { useBenchState } from "@/state/bench";
import { VERSION } from "@/utils/globals";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { ArrowsPointingOutIcon, Bars3BottomLeftIcon, MapIcon, MoonIcon, WindowIcon } from "@heroicons/vue/24/outline";

const appearance = useAppearance();
const bench = useBenchState();

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
        class="absolute bottom-1 left-11 z-40 flex w-60 flex-col gap-2 rounded-sm bg-white px-2 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Version -->
        <span class="px-2 text-center text-xs text-gray-700">Bench {{ VERSION }}</span>
        <!-- Font style -->
        <div class="w-full px-2 pb-1">
          <!-- <span class="text-xs text-gray-500">Style</span> -->
          <div class="flex flex-row justify-between">
            <button
              v-for="font in fontOptions"
              :key="font.key"
              class="flex flex-col items-center justify-center rounded-sm px-5 text-center hover:bg-orange-100"
              :class="font.style"
              @click="appearance.font = font.key as Font"
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
          <Switch v-model="appearance.contentWide" />
        </div>
        <!-- Panel headers -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <WindowIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Panel tabs</span>
          </span>
          <Switch v-model="bench.showPanelTabs" />
        </div>
        <!-- Panel headers -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <WindowIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Panel explorer</span>
          </span>
          <Switch v-model="bench.showPanelExplorer" />
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
            disabled
          />
        </div>
        <!-- Zen mode -->
        <!-- (should become an action) -->
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
