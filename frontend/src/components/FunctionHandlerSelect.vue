<template>
  <Combobox as="div" v-model="selectedHandler">
    <ComboboxLabel v-if="label" class="block text-sm font-medium text-gray-700">
      {{ label }}
    </ComboboxLabel>
    <div class="relative mt-1">
      <ComboboxInput
        class="w-full rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 shadow-sm focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 sm:text-sm"
        @change="query = $event.target.value"
        :display-value="(handler) => handler?.name"
      />
      <ComboboxButton
        class="absolute inset-y-0 right-0 flex items-center rounded-r-md px-2 focus:outline-none"
      >
        <SelectorIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
      </ComboboxButton>

      <ComboboxOptions
        v-if="filteredHandlers.length > 0"
        class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
      >
        <ComboboxOption
          v-for="handler in filteredHandlers"
          :key="handler.name"
          :value="handler"
          as="template"
          v-slot="{ active, selected }"
        >
          <li
            :class="[
              'relative cursor-default select-none py-2 pl-8 pr-4',
              active ? 'bg-orange-600 text-white' : 'text-gray-900',
            ]"
          >
            <span :class="['block truncate', selected && 'font-semibold']">
              {{ handler.name }}
            </span>

            <span
              v-if="selected"
              :class="[
                'absolute inset-y-0 left-0 flex items-center pl-1.5',
                active ? 'text-white' : 'text-orange-600',
              ]"
            >
              <CheckIcon class="h-5 w-5" aria-hidden="true" />
            </span>
          </li>
        </ComboboxOption>
      </ComboboxOptions>
    </div>
  </Combobox>
</template>
<script lang="ts" setup>
import { computed, ref } from "vue";
import { CheckIcon, SelectorIcon } from "@heroicons/vue/solid";
import {
  Combobox,
  ComboboxButton,
  ComboboxInput,
  ComboboxLabel,
  ComboboxOption,
  ComboboxOptions,
} from "@headlessui/vue";
import { useMetaStore } from "@/stores";

defineProps<{ label?: string }>();

const metaStore = useMetaStore();

const query = ref("");
const selectedHandler = ref();
const filteredHandlers = computed(() =>
  query.value === ""
    ? metaStore.functionHandlers
    : metaStore.functionHandlers.filter((handler) => {
        return handler.name.toLowerCase().includes(query.value.toLowerCase());
      })
);
</script>
