<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActions } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { Popover, PopoverPanel, Switch } from "@headlessui/vue";

const editor = useEditorState();
const actions = useActions();
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute bottom-0 left-14 z-10 flex w-60 flex-col gap-2 rounded-sm bg-white px-4 pt-2 pb-4 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Font style -->
        <div class="pb-1">
          <span class="text-xs text-gray-500">Style</span>
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
        <div class="flex flex-row items-center justify-between">
          <span class="text-sm text-gray-900">Small text</span>
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
        <div class="flex flex-row items-center justify-between">
          <span class="text-sm text-gray-900">Line numbers</span>
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
        <div class="flex flex-row items-center justify-between">
          <span class="class text-sm text-gray-700">Dark mode - soon!</span>
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
        <div class="flex flex-row items-center justify-between">
          <span class="text-sm text-gray-900">Zen mode</span>
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
