<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { useActions } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { BellSlashIcon, CalculatorIcon, MinusCircleIcon, MoonIcon } from "@heroicons/vue/24/outline";

const editor = useEditorState();
const actions = useActions();
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute bottom-1 left-14 z-10 flex w-60 flex-col gap-2 rounded-sm bg-white px-2 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Font style -->
        <div class="px-2 pb-1">
          <!-- <span class="text-xs text-gray-500">Style</span> -->
          <div class="flex flex-row justify-center">
            <button
              class="flex flex-col items-center justify-center rounded-sm px-3 text-center hover:bg-orange-50"
              @click="editor.fontMono = false"
            >
              <span class="font-sans text-2xl" :class="{ 'text-orange-600': !editor.fontMono }">Ag</span>
              <span class="text-xs text-gray-500">Default</span>
            </button>
            <button
              class="flex flex-col items-center justify-center rounded-sm px-3 text-center hover:bg-orange-50"
              @click="editor.fontMono = true"
            >
              <span class="font-mono text-2xl" :class="{ 'text-orange-600': editor.fontMono }">Ag</span>
              <span class="text-xs text-gray-500">Mono</span>
            </button>
          </div>
        </div>
        <!-- Font size -->
        <div class="flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MinusCircleIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Small text</span>
          </span>
          <Switch v-model="editor.textSmall" />
        </div>
        <!-- Line numbers -->
        <div class="flex flex-row items-center justify-between px-2 py-1">
          <span class="flex flex-row items-center gap-2">
            <CalculatorIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Line numbers</span>
          </span>
          <Switch v-model="editor.showLineNumbers" />
        </div>
        <!-- Dark mode -->
        <div class="py--1 flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MoonIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Dark mode (soon)</span>
          </span>
          <Switch disabled v-model="editor.darkMode" />
        </div>
        <!-- Zen mode -->
        <div class="flex flex-row items-center justify-between px-2 py-1">
          <span class="flex flex-row items-center gap-2">
            <BellSlashIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Zen mode</span>
          </span>
          <Switch :model-value="editor.zenMode" @update:model-value="actions.apply('editor.zenMode')" />
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
