<script lang="ts" setup>
import AccessLevelSelect from "@/components/basic/AccessLevelSelect.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { ModuleAccessLevel, encodeSharingToken } from "@/state/auth";
import { PROJECT_ACCESS_LEVEL_NAME, useBenchState, type ProjectHeader } from "@/state/bench";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { LinkIcon, ShareIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const props = defineProps<{
  project: ProjectHeader;
}>();

const bench = useBenchState();
const notifications = useNotifications();
const ops = useOperations();

const projectSharingUrl = computed(() => {
  return `${document.location.origin}/${props.project.owner.slug}/${props.project.slug}?s=${encodeSharingToken(
    props.project.sharingToken
  )}`;
});
const cleanProjectSharingUrl = computed(() => {
  // without protocol or port
  return projectSharingUrl.value.replace(/^(https?:)?\/\//, "").replace(/:\d+/, "");
});
const projectUrl = computed(() => {
  return `${document.location.origin}/${props.project.owner.slug}/${props.project.slug}`;
});
const cleanProjectUrl = computed(() => {
  // without protocol or port
  return projectUrl.value.replace(/^(https?:)?\/\//, "").replace(/:\d+/, "");
});

function updateProjectSharing(sharing: { baseLevel?: ModuleAccessLevel; sharingLevel?: ModuleAccessLevel }) {
  ops.project.updateSharing(
    props.project.id,
    sharing.baseLevel ?? props.project.baseLevel,
    props.project.sharingEnabled,
    props.project.sharingToken,
    sharing.sharingLevel ?? props.project.sharingLevel
  );
}

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
        class="absolute right-0 top-10 z-30 flex w-[440px] flex-col gap-2 rounded-sm bg-white px-2 py-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <span class="px-2 font-bold">Share</span>

        <!-- You -->
        <div class="mx-2 rounded-sm bg-gray-100 px-3 py-2 text-sm text-gray-900">
          <span
            >You have
            <span class="font-semibold">{{
              PROJECT_ACCESS_LEVEL_NAME[project.accessLevel as ModuleAccessLevel].toLocaleLowerCase()
            }}</span>
            access to {{ project.slug }}.</span
          >
        </div>

        <!-- Settings -->
        <div class="mt-1 flex flex-col gap-y-1.5 px-2 text-gray-900">
          <!-- Other sharing settings will be configurable later (owner, org members, project members) -->
          <!-- Public link -->
          <div class="flex flex-col gap-y-0.5 py-0.5">
            <div class="flex flex-row justify-between px-1 py-1">
              <span>Everyone who can see this</span>
              <AccessLevelSelect
                :model-value="project.baseLevel"
                @update:model-value="(level) => updateProjectSharing({ baseLevel: level })"
                :readonly="!bench.canManage"
              />
            </div>
            <!-- Actual link -->
            <div class="flex flex-row gap-2 bg-gray-100 px-2 py-1.5">
              <span class="scroll-hidden w-full select-all overflow-x-scroll whitespace-nowrap text-gray-500">{{
                cleanProjectUrl
              }}</span>
              <button
                class="flex flex-shrink-0 flex-row items-center whitespace-nowrap rounded-sm p-0.5 hover:bg-orange-100"
                @click="copy(projectUrl), close()"
              >
                <LinkIcon class="mr-1 h-4 w-4 text-gray-500" />
              </button>
            </div>
          </div>
          <!-- Sharing link -->
          <div class="flex flex-col gap-y-0.5 py-0.5">
            <div class="flex flex-row justify-between px-1 py-1">
              <span>Everyone with the sharing link</span>
              <AccessLevelSelect
                :model-value="project.sharingLevel"
                @update:model-value="(level) => updateProjectSharing({ sharingLevel: level })"
                :readonly="!bench.canManage"
              />
            </div>
            <!-- Actual link -->
            <div class="flex flex-row gap-2 bg-gray-100 px-2 py-1.5">
              <span class="scroll-hidden w-full select-all overflow-x-scroll whitespace-nowrap text-gray-500">
                {{ cleanProjectSharingUrl }}
              </span>
              <button
                class="flex flex-shrink-0 flex-row items-center whitespace-nowrap rounded-sm p-0.5 hover:bg-orange-100"
                @click="copy(projectSharingUrl), close()"
              >
                <LinkIcon class="mr-1 h-4 w-4 text-gray-500" />
              </button>
            </div>
          </div>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
