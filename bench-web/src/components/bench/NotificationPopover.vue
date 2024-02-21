<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { NotificationStatus } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { BellIcon } from "@heroicons/vue/24/solid";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, watch, type Ref } from "vue";

const { activeCount, render, mark } = useNotifications();
const hasUnreadNotifications = computed(() => activeCount.value > 0);

const showingArchive: Ref<boolean> = ref(false);

// use reference to element inside PopoverPanel to determine if it's open
// once it's open we enable & refetch
const panelHeaderRef = ref<HTMLDivElement | null>(null);
watch(
  panelHeaderRef,
  () => {
    if (panelHeaderRef.value != null) {
      refetch();
    }
  },
  { deep: false }
);

const {
  result: notificationsResult,
  loading,
  refetch,
} = useQuery(
  graphql(/* GraphQL */ `
    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {
      me {
        id
        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {
          totalCount
          edges {
            node {
              id
              type
              createdAt
              readAt
              archivedAt
              expiresAt
              status
              organizationInvite {
                id
                organization {
                  id
                  slug
                  name
                }
                level
              }
              projectInvite {
                id
                project {
                  id
                  slug
                  name
                }
                level
              }
            }
          }
        }
      }
    }
  `),
  computed(() => ({
    status: showingArchive.value ? NotificationStatus.Archived : undefined,
    notArchived: !showingArchive.value,
    first: 10,
  })) as any,
  {
    enabled: computed(() => panelHeaderRef.value != null) as any,
  }
);
const notifications = computed(() => notificationsResult.value?.me?.notifications.edges.map((e) => e.node) ?? []);
const renderedNotifications = computed(() =>
  notifications.value.map((notification) => ({
    ...notification,
    ...render(notification as any),
  }))
);

async function markRead(notification: { id: string }) {
  await mark(notification.id, NotificationStatus.Read);
}

async function markUnread(notification: { id: string }) {
  await mark(notification.id, NotificationStatus.Active);
}

async function toggleRead(notification: { id: string; status: NotificationStatus }) {
  if (notification.status != NotificationStatus.Active) {
    await mark(notification.id, NotificationStatus.Active);
  } else {
    await mark(notification.id, NotificationStatus.Read);
  }
}

async function markArchived(notification: { id: string }) {
  await mark(notification.id, NotificationStatus.Archived);
}

const { getTimeFromNowString } = useTimeFromNow();
</script>
<template>
  <Popover v-slot="{ open }" as="div" class="relative">
    <!-- TODO :Cleanup: no idea why mt-1 is necessary on notifications popover button for horizontal alignment with other buttons -->
    <PopoverButton
      class="mt-1 rounded-sm p-1 text-sm text-orange-600 outline-none transition-colors hover:bg-orange-100"
      :class="{
        'bg-orange-100': open,
      }"
    >
      <BellIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-2 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <!-- Header -->
        <div class="" ref="panelHeaderRef">
          <h2 class="font-bold text-gray-900">Notifications</h2>
          <!-- Unread / archived toggle -->
          <!-- Dismiss all button etc. -->
        </div>
        <!-- Empty state -->
        <p class="text-gray-500" v-if="loading"></p>
        <p class="text-gray-500" v-else-if="notifications.length == 0">All caught up.</p>
        <!-- Notifications -->
        <div v-else class="flex w-full flex-col items-center gap-y-4 transition">
          <div
            v-for="notification in renderedNotifications"
            :key="notification.id"
            class="flex w-full flex-row items-baseline justify-between overflow-hidden rounded-sm border-l-2 py-1.5 pl-1.5 pr-2 hover:cursor-pointer hover:bg-orange-100"
            :class="{
              'border-orange-600': notification.status === NotificationStatus.Active,
              'border-gray-200': notification.status === NotificationStatus.Read,
              'border-gray-400': notification.status === NotificationStatus.Archived,
            }"
            @click="toggleRead(notification)"
          >
            <!-- Main message -->
            <div class="ml-1 flex flex-col">
              <h3
                class="relative flex flex-row items-center gap-1 text-sm text-gray-900"
                :class="{ 'font-bold': notification.status == NotificationStatus.Active }"
              >
                <component
                  :is="notification.icon"
                  v-if="notification.icon"
                  class="absolute h-4 w-4"
                  :class="notification.status == NotificationStatus.Active ? 'text-gray-900' : 'text-gray-500'"
                />
                <span class="ml-5">{{ notification.message }}</span>
              </h3>
              <p v-if="notification.description" class="text-xs text-gray-500">{{ notification.description }}</p>
            </div>
            <!-- Actions & Time -->
            <div class="flex flex-row items-baseline">
              <button
                v-if="notification.actionText"
                type="button"
                class="h-fit flex-shrink-0 rounded-sm px-3 text-sm font-medium text-gray-900 decoration-gray-500 decoration-dashed underline-offset-4 hover:underline hover:decoration-gray-900 hover:decoration-solid focus:outline-none"
                @click="() => notification.action?.()"
              >
                {{ notification.actionText }}
              </button>
              <span class="text-gray-500">{{ getTimeFromNowString(notification.createdAt) }}</span>
            </div>
          </div>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
