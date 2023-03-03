<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActions } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { Popover, PopoverPanel, Switch } from "@headlessui/vue";
import { BellSlashIcon, BellSnoozeIcon, CalculatorIcon, MinusCircleIcon, MoonIcon } from "@heroicons/vue/24/outline";

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
          <Switch
            v-model="editor.textSmall"
            :class="[
              editor.textSmall ? 'bg-orange-600' : 'bg-gray-200',
              'relative inline-flex h-4 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out focus:outline-none',
            ]"
          >
            <span
              aria-hidden="true"
              :class="[
                editor.textSmall ? 'translate-x-5' : 'translate-x-0',
                'pointer-events-none inline-block h-3 w-3 transform rounded-full bg-white shadow ring-0 transition duration-100 ease-in-out',
              ]"
            />
          </Switch>
        </div>
        <!-- Line numbers -->
        <div class="flex flex-row items-center justify-between px-2 py-1">
          <span class="flex flex-row items-center gap-2">
            <CalculatorIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Line numbers</span>
          </span>
          <Switch
            v-model="editor.showLineNumbers"
            :class="[
              editor.showLineNumbers ? 'bg-orange-600' : 'bg-gray-200',
              'relative inline-flex h-4 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out focus:outline-none',
            ]"
          >
            <span
              aria-hidden="true"
              :class="[
                editor.showLineNumbers ? 'translate-x-5' : 'translate-x-0',
                'pointer-events-none inline-block h-3 w-3 transform rounded-full bg-white shadow ring-0 transition duration-100 ease-in-out',
              ]"
            />
          </Switch>
        </div>
        <!-- Dark mode -->
        <div class="py--1 flex flex-row items-center justify-between px-2">
          <span class="flex flex-row items-center gap-2">
            <MoonIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Dark mode (soon)</span>
          </span>
          <Switch
            disabled
            v-model="editor.darkMode"
            :class="[
              editor.darkMode ? 'bg-orange-600' : 'bg-gray-200',
              'relative inline-flex h-4 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out focus:outline-none',
            ]"
          >
            <span
              aria-hidden="true"
              :class="[
                editor.darkMode ? 'translate-x-5' : 'translate-x-0',
                'pointer-events-none inline-block h-3 w-3 transform rounded-full bg-white shadow ring-0 transition duration-100 ease-in-out',
              ]"
            />
          </Switch>
        </div>
        <!-- Zen mode -->
        <div class="flex flex-row items-center justify-between px-2 py-1">
          <span class="flex flex-row items-center gap-2">
            <BellSlashIcon class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">Zen mode</span>
          </span>
          <Switch
            :model-value="editor.zenMode"
            @update:model-value="actions.apply('editor.zenMode')"
            :class="[
              editor.zenMode ? 'bg-orange-600' : 'bg-gray-200',
              'relative inline-flex h-4 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out focus:outline-none',
            ]"
          >
            <span
              aria-hidden="true"
              :class="[
                editor.zenMode ? 'translate-x-5' : 'translate-x-0',
                'pointer-events-none inline-block h-3 w-3 transform rounded-full bg-white shadow ring-0 transition duration-100 ease-in-out',
              ]"
            />
          </Switch>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
