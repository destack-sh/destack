<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { ModuleAccessLevel } from "@/gql/graphql";
import { encodeSharingToken } from "@/state/auth";
import { type ProjectHeader } from "@/state/bench";
import { useNotifications } from "@/state/notifications";
import {
  Listbox,
  ListboxButton,
  ListboxOption,
  ListboxOptions,
  Popover,
  PopoverButton,
  PopoverPanel,
} from "@headlessui/vue";
import { ChevronUpDownIcon, LinkIcon, ShareIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const props = defineProps<{
  project: ProjectHeader;
}>();

const notifications = useNotifications();

const projectSharingUrl = computed(() => {
  return `${document.location.origin}/${props.project.owner.slug}/${props.project.slug}?s=${encodeSharingToken(
    props.project.sharingToken
  )}`;
});
const cleanProjectSharingUrl = computed(() => {
  // without protocol or port
  return projectSharingUrl.value.replace(/^(https?:)?\/\//, "").replace(/:\d+/, "");
});

function copy(text: string) {
  navigator.clipboard.writeText(text);
  notifications.show({
    kind: "success",
    type: "sharing.copied",
    message: "Sharing link copied to clipboard",
  });
}
</script>

<template>
  <Popover v-slot="{ open, close }" class="relative">
    <PopoverButton
      class="flex flex-row items-center rounded-sm px-1 py-1 text-sm focus:outline-none"
      :class="{
        'hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <ShareIcon class="h-5 w-5 text-orange-600" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 flex w-96 flex-col gap-2 rounded-sm bg-white px-2 py-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <span class="px-2 font-bold">Share</span>

        <!-- You -->
        <div class="mx-2 rounded-sm bg-gray-100 px-3 py-2 text-sm text-gray-900">
          <span
            >You have <span class="font-semibold">{{ project.accessLevel.toLocaleLowerCase() }}</span> access to
            {{ project.slug }}.</span
          >
        </div>

        <!-- Settings -->
        <div class="mt-1 flex flex-col gap-2 px-2 text-gray-900">
          <!-- Other sharing settings will be configurable later (owner, org members, project members) -->
          <!-- Sharing link -->
          <div class="flex flex-col gap-1.5 py-0.5">
            <div class="flex flex-row justify-between px-1 py-1 hover:bg-gray-100">
              <span>Everyone with the sharing link</span>
              <Listbox as="div" class="relative" v-slot="{ open }" :model-value="project.sharingLevel">
                <ListboxButton
                  class="flex flex-row items-center text-left text-sm text-gray-700 hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
                  :class="[open ? 'bg-orange-100' : '']"
                >
                  <ChevronUpDownIcon class="mr-0.5 h-4 w-4 text-gray-500" />
                  Can {{ project.sharingLevel.toLocaleLowerCase() }}
                </ListboxButton>
                <FadeTransition>
                  <ListboxOptions
                    class="absolute right-1 top-6 z-30 w-52 origin-top-right rounded-sm bg-white px-2 py-1.5 shadow-md ring-1 ring-orange-900 ring-opacity-40"
                  >
                    <ListboxOption
                      v-for="level in [ModuleAccessLevel.Read, ModuleAccessLevel.Use, ModuleAccessLevel.Edit]"
                      :key="level"
                      as="div"
                      class="flex flex-col p-1 hover:bg-orange-100 focus:bg-orange-100"
                      :class="[project.sharingLevel === level ? 'text-orange-600' : '']"
                    >
                      <span> Can {{ level.toLocaleLowerCase() }} </span>
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
            </div>
            <!-- Actual link -->
            <div class="flex flex-row gap-2">
              <span
                class="scroll-hidden w-full select-all overflow-x-scroll whitespace-nowrap bg-gray-100 px-2 py-1.5 text-gray-500"
                >{{ cleanProjectSharingUrl }}</span
              >
              <button
                class="flex flex-shrink-0 flex-row items-center whitespace-nowrap rounded-sm p-1 hover:bg-orange-100"
                @click="copy(projectSharingUrl), close()"
              >
                <LinkIcon class="mr-1 h-4 w-4 text-gray-500" /><span class="text-sm">Copy</span>
              </button>
            </div>
          </div>
          <!-- Project members (soon) -->
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
