<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useFragment, type FragmentType } from "@/gql";
import { ProjectVisibility } from "@/gql/graphql";
import { ProjectHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { Popover, PopoverPanel } from "@headlessui/vue";
import { ArrowRightOnRectangleIcon, GlobeAltIcon, LockClosedIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();
const project = computed(() => useFragment(ProjectHeaderType, props.project));

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
  await ops.project.updateVisibility(project.value.id, visibility);

  notifications.dismissIf({ type: "project.visibilityUpdated" });
  notifications.show({
    kind: "success",
    type: "project.visibilityUpdated",
    message: visibility == ProjectVisibility.Public ? "Bench is public" : "Bench is private",
    description:
      visibility == ProjectVisibility.Public
        ? "Everyone can see and copy this Bench."
        : `Only ${project.value.owner.slug} can work this Bench.`,
  });
}
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute top-10 left-0 z-10 flex w-52 flex-col gap-1 rounded-sm bg-white px-2 pt-2 pb-4 shadow-md ring-1 ring-orange-900 ring-opacity-40"
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
              class="flex flex-row items-center justify-center gap-1 rounded-sm py-1 px-3 text-center hover:bg-orange-50"
              :class="{ 'text-orange-600': project.visibility === ProjectVisibility.Private }"
              @click="updateVisibility(ProjectVisibility.Private)"
            >
              <LockClosedIcon class="h-5 w-5" />
              <span>Private</span>
            </button>
            <button
              class="flex flex-row items-center justify-center gap-1 rounded-sm py-1 px-3 text-center hover:bg-orange-50"
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
          class="flex flex-row items-center gap-1.5 py-1 px-2 text-left text-sm"
          :class="{ 'cursor-not-allowed text-gray-500': !action.enabled, 'hover:bg-orange-50': action.enabled }"
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
