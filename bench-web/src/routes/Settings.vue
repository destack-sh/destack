<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationPopover from "@/components/bench/NotificationPopover.vue";
import SettingsAccessTokens from "@/components/settings/SettingsAccessTokens.vue";
import SettingsMembers from "@/components/settings/SettingsMembers.vue";
import SettingsProfile from "@/components/settings/SettingsProfile.vue";
import { graphql } from "@/gql";
import { useNotifications } from "@/state/notifications";
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/vue";
import {
  BellIcon,
  Cog8ToothIcon,
  CogIcon,
  CreditCardIcon,
  KeyIcon,
  UserCircleIcon,
  UserGroupIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed, ref, watchEffect } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{ owner: string }>();

const { result: settingsResult, loading } = useQuery(
  graphql(/* GraphQL */ `
    query settings($slug: String!) {
      ownerBySlug(slug: $slug) {
        ... on User {
          id
          slug
          name
          username
          bot
          createdAt
          updatedAt
          canViewDetail
          canWrite
          accessTokens(filters: { includeInactive: false }) {
            totalCount
          }
        }
        ... on Organization {
          id
          slug
          name
          createdAt
          updatedAt
          canViewDetail
          canWrite
          memberships {
            totalCount
          }
          accessTokens(filters: { includeInactive: false }) {
            totalCount
          }
        }
      }
    }
  `),
  computed(() => ({ slug: props.owner }))
);
const profile = computed(() => settingsResult.value?.ownerBySlug);
const user = computed(() => (profile.value?.__typename === "User" ? profile.value : null));
const organization = computed(() => (profile.value?.__typename === "Organization" ? profile.value : null));

// sync title
const title = useTitle();
watchEffect(() => {
  if (profile.value == null && loading.value) {
    title.value = props.owner;
  } else if (profile.value != null) {
    title.value = `${props.owner} • Settings`;
  } else {
    // not found
    title.value = "Page not found";
  }
});

type SettingsTab = {
  id: string;
  name: string;
  icon: unknown;
  component?: unknown;
  disabled?: boolean;
  count?: number;
};

const tabs = computed(() => {
  const tabs: SettingsTab[] = [
    {
      id: "profile",
      name: "Profile",
      icon: UserCircleIcon,
      component: SettingsProfile,
    },
  ];
  if (user.value != null) {
    tabs.push({
      id: "account",
      name: "Account",
      icon: CogIcon,
      disabled: true,
    });
  } else {
    // organization
    tabs.push({
      id: "members",
      name: "Members",
      icon: UserGroupIcon,
      count: organization.value?.memberships.totalCount ?? 0,
      component: SettingsMembers,
    });
  }
  tabs.push({
    id: "notifications",
    name: "Notifications",
    icon: BellIcon,
    disabled: true,
  });
  tabs.push({
    id: "access-tokens",
    name: "Access tokens",
    icon: KeyIcon,
    count: profile.value?.accessTokens.totalCount ?? 0,
    component: SettingsAccessTokens,
  });
  tabs.push({
    id: "plan",
    name: "Plan & billing",
    icon: CreditCardIcon,
    disabled: true,
  });
  return tabs;
});

const router = useRouter();

// sync selected settings tab with router hash
const selectedTab = ref(router.currentRoute.value.hash.replace("#", ""));
const selectedIndex = computed(() => tabs.value.findIndex((tab) => tab.id === selectedTab.value));
function selectTab(index: number) {
  selectedTab.value = tabs.value[index].id;
  router.replace({ hash: `#${tabs.value[index].id}` });
}
// reset to first tab if selectedIndex is invalid
if (selectedIndex.value < 0) {
  selectTab(0);
}

// redirect to public profile page if can't view full
const notifications = useNotifications();
watchEffect(() => {
  if (profile.value != null && !profile.value.canViewDetail) {
    router.push(`/${props.owner}`);
    notifications.show({
      kind: "notice",
      type: "auth.cantView",
      message: "Can't view this",
      description: "The robots have decreed you're not allowed there.",
    });
  }
});
</script>

<template>
  <div class="flex h-full w-full flex-col bg-gray-50">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <NotificationPopover />
        <OmniCreate class="ml-2" />
        <ProfileButton class="ml-2" />
      </template>
    </FatHeader>
    <main
      class="mx-auto mt-8 flex w-full max-w-[1100px] flex-grow flex-col px-8 md:gap-8 md:py-4 lg:flex-row lg:items-baseline"
      v-if="profile != null"
    >
      <TabGroup :selected-index="selectedIndex" @change="selectTab" as="template">
        <!-- Profile info -->
        <div class="flex w-80 flex-col gap-2">
          <!-- Name / username -->
          <div class="border-b-2 border-orange-900/[12%] pb-2">
            <h1 class="flex max-w-full flex-row items-center gap-2 text-gray-900">
              <router-link
                :to="`/${profile.slug}`"
                class="truncate text-2xl font-bold underline-offset-4 hover:underline"
              >
                {{ user?.name || organization?.name }}
              </router-link>
              <Cog8ToothIcon class="h-6 w-6 text-gray-500" />
            </h1>
            <router-link :to="`/${profile.slug}`" class="text-xl text-gray-700 underline-offset-4 hover:underline">
              {{ profile.slug }}
            </router-link>
          </div>
          <!-- Settings tabs -->
          <TabList class="flex flex-col gap-1 text-left">
            <Tab v-for="tab in tabs" :key="tab.name" v-slot="{ selected }" as="template" :disabled="tab.disabled">
              <button
                class="relative flex flex-row items-baseline gap-1 rounded-sm border-l-2 px-2 py-1 focus:outline-none"
                :class="{
                  'border-orange-600 bg-orange-100 text-orange-600': selected,
                  'border-transparent text-gray-700 ': !selected && !tab.disabled,
                  'border-transparent text-gray-400': !selected && tab.disabled,
                }"
              >
                <component
                  :is="tab.icon"
                  class="absolute top-1.5 h-5 w-5"
                  :class="{
                    'text-gray-700': !selected && !tab.disabled,
                    'text-gray-400': !selected && tab.disabled,
                    'text-orange-600': selected,
                  }"
                />
                <span class="ml-7 px-0.5">{{ tab.name }}</span>
                <span
                  v-if="tab.count != null"
                  class="rounded-3xl px-1.5 text-sm"
                  :class="selected ? 'bg-orange-200 text-orange-900' : 'bg-gray-100'"
                  >{{ tab.count }}</span
                >
              </button>
            </Tab>
          </TabList>
        </div>
        <TabPanels class="h-full w-full">
          <TabPanel v-for="tab in tabs" :key="tab.name">
            <h3 class="text-2xl font-bold text-gray-900">{{ tab.name }}</h3>
            <component class="mt-4" v-if="tab.component" :is="tab.component" :slug="props.owner" />
          </TabPanel>
        </TabPanels>
      </TabGroup>
    </main>
    <GenericNotFound v-if="!loading && profile == null" class="flex-grow pb-12" />
    <NotificationArea />
  </div>
</template>
