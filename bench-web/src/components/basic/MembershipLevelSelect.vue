<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { OrganizationRole } from "@/gql/graphql";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { ChevronDownIcon, EyeIcon, PencilIcon, WalletIcon, WrenchScrewdriverIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ modelValue?: OrganizationRole }>();
const emit = defineEmits<{ (e: "update:modelValue", value: OrganizationRole): void }>();

const levels = [
  {
    value: OrganizationRole.Guest,
    label: "Guest",
    icon: EyeIcon,
    description: "View and comment",
  },
  {
    value: OrganizationRole.Member,
    label: "Member",
    icon: PencilIcon,
    description: "Create and edit Benches",
  },
  {
    value: OrganizationRole.Administrator,
    label: "Admin",
    icon: WrenchScrewdriverIcon,
    description: "Manage settings",
  },
  {
    value: OrganizationRole.Owner,
    label: "Owner",
    icon: WalletIcon,
    description: "Do anything",
  },
];

const selected = computed(() => {
  if (props.modelValue == null) {
    return null;
  }
  return levels.find((level) => level.value === props.modelValue);
});
</script>
<template>
  <Listbox
    as="div"
    class="relative"
    nullable
    :model-value="selected"
    by="value"
    @update:model-value="emit('update:modelValue', $event.value)"
    v-slot="{ open }"
  >
    <slot name="button">
      <ListboxButton
        class="flex flex-row items-center gap-1 py-1 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
        :class="open ? 'bg-orange-100' : ''"
      >
        <p class="text-sm">{{ selected?.label }}</p>
        <ChevronDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
      </ListboxButton>
    </slot>
    <FadeTransition>
      <ListboxOptions
        class="absolute left-0 top-8 z-10 mt-0 w-56 rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
      >
        <ListboxOption
          v-for="level in levels"
          :key="level.value"
          :value="level"
          v-slot="{ active, selected }"
          as="template"
        >
          <div
            class="flex flex-row items-center gap-3 text-left hover:cursor-pointer"
            :class="[
              active ? 'bg-orange-100' : '',
              'block px-2 py-1.5 text-sm text-gray-900',
              selected ? 'text-orange-600' : '',
            ]"
          >
            <span
              class="rounded-sm p-1.5"
              :class="selected ? 'bg-orange-100 text-orange-800 ' : 'bg-gray-100 text-gray-700'"
            >
              <component :is="level.icon" class="h-5 w-5" />
            </span>
            <span class="flex flex-col">
              <span>{{ level.label }}</span>
              <span class="text-xs text-gray-500">{{ level.description }}</span>
            </span>
          </div>
        </ListboxOption>
      </ListboxOptions>
    </FadeTransition>
  </Listbox>
</template>
