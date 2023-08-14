<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { ProjectVisibility, type Project } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { ArrowRightOnRectangleIcon, GlobeAltIcon, LockClosedIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ project: Pick<Project, "id" | "name" | "slug" | "visibility"> }>();

const projectActions = computed(() => [
  {
    name: "Move",
    icon: ArrowRightOnRectangleIcon,
    enabled: false,
    action: () => ({}),
  },
]);

const ops = useOperations();
const notifications = useNotifications();
async function updateVisibility(visibility: ProjectVisibility) {
  await ops.project.updateVisibility(props.project.id, visibility);

  notifications.dismissIf({ type: "project.visibilityUpdated" });
  notifications.show({
    kind: "success",
    type: "project.visibilityUpdated",
    message: visibility == ProjectVisibility.Public ? "Bench is public" : "Bench is private",
    description:
      visibility == ProjectVisibility.Public
        ? "Everyone can see and copy this Bench."
        : `Only ${props.project.slug} can work on this Bench.`,
  });
}
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute left-0 top-10 z-30 flex w-52 flex-col gap-1 rounded-sm bg-white px-2 pb-4 pt-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Name -->
        <div class="flex max-w-full flex-col">
          <h2 class="truncate px-2">{{ project.name }}</h2>
          <h3 class="truncate px-2 text-xs text-gray-500">{{ project.slug }}</h3>
        </div>
        <!-- Visibility -->
        <div class="px-2">
          <div class="flex flex-row justify-center py-1 text-sm">
            <button
              class="flex flex-row items-center justify-center gap-1 rounded-sm px-3 py-1 text-center hover:bg-orange-100"
              :class="{ 'text-orange-600': project.visibility === ProjectVisibility.Private }"
              @click="updateVisibility(ProjectVisibility.Private)"
            >
              <LockClosedIcon class="h-5 w-5" />
              <span>Private</span>
            </button>
            <button
              class="flex flex-row items-center justify-center gap-1 rounded-sm px-3 py-1 text-center hover:bg-orange-100"
              :class="{ 'text-orange-600': project.visibility === ProjectVisibility.Public }"
              @click="updateVisibility(ProjectVisibility.Public)"
            >
              <GlobeAltIcon class="h-5 w-5" />
              <span>Public</span>
            </button>
          </div>
        </div>
        <!-- Actions -->
        <button
          v-for="action in projectActions"
          :key="action.name"
          class="flex flex-row items-center gap-1.5 px-2 py-1 text-left text-sm"
          :class="{ 'cursor-not-allowed text-gray-500': !action.enabled, 'hover:bg-orange-100': action.enabled }"
          :disabled="!action.enabled"
        >
          <component :is="action.icon" class="h-4 w-4 text-gray-700" />
          <span>
            {{ action.name }}
          </span>
        </button>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
