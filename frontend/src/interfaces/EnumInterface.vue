<template>
  <Listbox
    as="div"
    class="w-full"
    :model-value="modelValue"
    @update:model-value="(value) => $emit('update:modelValue', value)"
  >
    <div class="relative mt-1">
      <ListboxButton
        class="relative w-full cursor-default rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 text-left shadow-sm focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 sm:text-sm"
      >
        <span class="block truncate">{{ modelValue }}</span>
        <span class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronUpDownIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
        </span>
      </ListboxButton>

      <transition
        leave-active-class="transition ease-in duration-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <ListboxOptions
          class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
        >
          <ListboxOption
            as="template"
            v-for="item in values"
            :key="item.id"
            :value="item"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                active ? 'bg-orange-600 text-white' : 'text-gray-900',
                'relative cursor-default select-none py-2 pl-3 pr-9',
              ]"
            >
              <span :class="[selected ? 'font-semibold' : 'font-normal', 'block truncate']">
                {{ item }}
              </span>

              <span
                v-if="selected"
                :class="[
                  active ? 'text-white' : 'text-orange-600',
                  'absolute inset-y-0 right-0 flex items-center pr-4',
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
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { CheckIcon, ChevronUpDownIcon } from "@heroicons/vue/24/solid";
import { ref, watchEffect } from "vue";

const props = defineProps<{
  modelValue?: string;
  values?: any[];
  default?: any;
}>();

const emit = defineEmits(["update:modelValue"]);

const setDefault = ref(false);
watchEffect(() => {
  if (setDefault.value) {
    return;
  }
  setDefault.value = true;
  if (props.modelValue == null) {
    if (props.default != null) {
      emit("update:modelValue", props.default);
    } else if (props.values != null && props.values.length > 0) {
      emit("update:modelValue", props.values[0]);
    }
  }
});
</script>
