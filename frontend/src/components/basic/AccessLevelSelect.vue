<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { ModuleAccessLevel } from "@/state/auth";
import { PROJECT_ACCESS_LEVEL_NAME } from "@/state/bench";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { ChevronUpDownIcon } from "@heroicons/vue/24/solid";

const props = defineProps<{
  readonly?: boolean;
  modelValue: ModuleAccessLevel;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: ModuleAccessLevel): void;
}>();
</script>
<template>
  <Listbox
    as="div"
    class="relative"
    v-slot="{ open }"
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :disabled="props.readonly"
  >
    <ListboxButton
      class="flex flex-row items-center text-left text-sm text-gray-700 hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
      :class="[open ? 'bg-orange-100' : '']"
    >
      <ChevronUpDownIcon class="mr-0.5 h-4 w-4 text-gray-500" />
      can {{ PROJECT_ACCESS_LEVEL_NAME[modelValue].toLowerCase() }}
    </ListboxButton>
    <FadeTransition>
      <ListboxOptions
        class="absolute right-1 top-6 z-30 w-52 origin-top-right rounded-sm bg-white px-2 py-1.5 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <ListboxOption
          v-for="level in [ModuleAccessLevel.Read, ModuleAccessLevel.Use, ModuleAccessLevel.Edit]"
          :key="level"
          :value="level"
          as="div"
          class="flex flex-col p-1 hover:cursor-pointer hover:bg-orange-100 focus:bg-orange-100"
          :class="[modelValue === level ? 'text-orange-600' : '']"
        >
          <span> can {{ PROJECT_ACCESS_LEVEL_NAME[level].toLowerCase() }} </span>
          <span class="text-xs text-gray-400">{{
            {
              [ModuleAccessLevel.Zero]: "Do and see nothing.",
              [ModuleAccessLevel.Read]: "View and comment, but not run.",
              [ModuleAccessLevel.Use]: "Use and read, but not edit.",
              [ModuleAccessLevel.Edit]: "Edit and use, but not manage.",
              [ModuleAccessLevel.Manage]: "Manage members, but not destruct.",
              [ModuleAccessLevel.Admin]: "Do everything.",
            }[level]
          }}</span>
        </ListboxOption>
      </ListboxOptions>
    </FadeTransition>
  </Listbox>
</template>
