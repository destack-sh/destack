<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { OrganizationRole } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { ChevronDownIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

defineProps<{ modelValue: { id: string; name: string; slug: string } | null }>();
const emit = defineEmits<{ (e: "update:modelValue", value: { id: string; name: string; slug: string }): void }>();

const auth = useAuth();
const possibleOwners = computed(() => {
  if (auth.me.value == null) {
    return [];
  } else {
    const writableOrgs =
      auth.memberships.value
        ?.filter((m) =>
          [OrganizationRole.Owner, OrganizationRole.Administrator, OrganizationRole.Member].includes(m.level)
        )
        .map((e) => e.organization) ?? [];
    return [auth.me.value, ...writableOrgs];
  }
});
</script>
<template>
  <div>
    <Listbox
      as="div"
      class="relative"
      nullable
      :model-value="modelValue"
      by="value"
      @update:model-value="emit('update:modelValue', $event)"
      v-slot="{ open }"
    >
      <slot name="button" :open="open">
        <ListboxButton
          class="flex flex-row items-center gap-1 py-1 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
          :class="open ? 'bg-orange-100' : ''"
        >
          <p class="text-sm">{{ modelValue?.slug }}</p>
          <ChevronDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
        </ListboxButton>
      </slot>
      <FadeTransition>
        <ListboxOptions
          class="absolute left-0 top-8 z-10 mt-0 max-h-64 w-56 overflow-y-auto rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
        >
          <ListboxOption
            v-for="owner in possibleOwners"
            :key="owner.id"
            :value="owner"
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
              <span class="flex flex-col">
                <span>{{ owner.name }}</span>
                <span class="text-xs text-gray-500">{{ owner.slug }}</span>
              </span>
            </div>
          </ListboxOption>
        </ListboxOptions>
      </FadeTransition>
    </Listbox>
  </div>
</template>
