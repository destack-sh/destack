<template>
  <Listbox as="div" :model-value="modelValue || {}" @update:model-value="(value) => $emit('update:modelValue', value)">
    <ListboxLabel class="mt-2 block text-sm font-medium text-gray-700"> Model handler </ListboxLabel>
    <div class="relative mt-1">
      <ListboxButton
        class="relative w-full cursor-default rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 text-left shadow-sm focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 sm:text-sm"
      >
        <span class="block truncate" :class="modelValue == null ? 'text-gray-500' : ''">
          {{ modelValue?.name || "Select a handler" }}
        </span>
        <span class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronUpDownIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
        </span>
      </ListboxButton>

      <transition
        leave-active-class="transition duration-100 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <ListboxOptions
          class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
        >
          <ListboxOption
            as="template"
            v-for="handler in metaStore.modelHandlers"
            :key="handler.name"
            :value="handler"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                active ? 'bg-orange-600 text-white' : 'text-gray-900',
                'relative cursor-default select-none py-2 pl-8 pr-4',
              ]"
            >
              <span :class="[selected ? 'font-semibold' : 'font-normal', 'block truncate']">
                {{ handler.name }}
              </span>

              <span
                v-if="selected"
                :class="[
                  active ? 'text-white' : 'text-orange-600',
                  'absolute inset-y-0 left-0 flex items-center pl-1.5',
                ]"
              >
                <CheckIcon class="h-5 w-5" aria-hidden="true" />
              </span>
            </li>
          </ListboxOption>
        </ListboxOptions>
      </transition>
    </div>
  </Listbox>
</template>
<script lang="ts" setup>
import { useMetaStore } from "@/stores";
import type { ModelHandlerSpec } from "@/types/spec.js";
import { Listbox, ListboxButton, ListboxLabel, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { CheckIcon, ChevronUpDownIcon } from "@heroicons/vue/24/solid";

const metaStore = useMetaStore();

defineProps<{ modelValue: ModelHandlerSpec | null }>();
defineEmits<{ (e: "update:modelValue", value: ModelHandlerSpec): void }>();
</script>
