<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { graphql } from "@/gql";
import { NotificationStatus } from "@/gql/graphql";
import { useNotifications } from "@/state/notifications";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { BellIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, type Ref } from "vue";

const { activeCount } = useNotifications();
const hasUnreadNotifications = computed(() => activeCount.value > 0);

const filterStatus: Ref<NotificationStatus> = ref(NotificationStatus.Active);
const open = ref(true); // TODO @Broken: sync this

const {
  result: notificationsResult,
  loading,
  refetch,
} = useQuery(
  graphql(/* GraphQL */ `
    query notifications($status: NotificationStatus, $first: Int) {
      me {
        notifications(filters: { status: $status }, first: $first) {
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
              invite {
                id
                organization {
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
    status: filterStatus.value,
    first: 10,
  })) as any,
  {
    enabled: open,
  }
);
const notifications = computed(() => notificationsResult.value?.me?.notifications.edges.map((e) => e.node) ?? []);
</script>
<template>
  <Popover v-slot="{ open }" as="div" class="relative">
    <PopoverButton
      class="rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-gray-500 hover:bg-orange-50': !hasUnreadNotifications,
        'text-orange-600 hover:bg-orange-50': hasUnreadNotifications,
        'bg-orange-50': open,
      }"
    >
      <BellIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-10 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pt-2 pb-4 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div>
          <h2 class="font-bold text-gray-900">Notifications</h2>
          <!-- Unread / archived toggle -->
          <!-- Dismiss all button -->
        </div>
        <!-- Empty state -->
        <p class="text-gray-500" v-if="loading"></p>
        <p class="text-gray-500" v-else-if="notifications.length == 0">Nothing here.</p>
        <!-- Notifications -->
        <ul v-else class="flex flex-col">
          <li v-for="notification in notifications" :key="notification.id">
            {{ notification.type }}
          </li>
        </ul>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
