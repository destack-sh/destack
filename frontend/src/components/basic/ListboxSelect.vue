<script lang="ts" setup>
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { ChevronUpDownIcon } from "@heroicons/vue/24/solid";

type Option = { name: string; value: string };

defineProps<{ modelValue: Option; options: Option[]; label?: string; emptyText?: string }>();
defineEmits<{ (e: "update:modelValue", value: Option): void }>();
</script>

<template>
  <Listbox as="div" :model-value="modelValue" @update:model-value="(value) => $emit('update:modelValue', value)">
    <div class="relative">
      <ListboxButton
        class="relative w-full cursor-default rounded-sm border border-white bg-white pr-6 text-left focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500"
      >
        <span class="block truncate" :class="modelValue == null ? 'text-gray-500' : 'text-gray-700'">
          {{ modelValue?.name || emptyText }}
        </span>
        <span class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronUpDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
        </span>
      </ListboxButton>

      <transition
        leave-active-class="transition duration-100 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <ListboxOptions
          class="absolute z-10 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
        >
          <ListboxOption
            as="template"
            v-for="option in options"
            :key="option.name"
            :value="option"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                active ? 'bg-gray-100 text-gray-700' : 'text-gray-700',
                'relative cursor-default select-none px-2 py-1 text-xs',
              ]"
            >
              <slot name="option" :option="option" :active="active" :selected="selected">
                <span :class="[selected ? 'font-semibold' : 'font-normal', 'block truncate']">
                  {{ option.name }}
                </span>
              </slot>
            </li>
          </ListboxOption>
        </ListboxOptions>
      </transition>
    </div>
  </Listbox>
</template>
