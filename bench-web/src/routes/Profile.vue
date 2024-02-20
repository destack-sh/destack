<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import NotificationPopover from "@/components/bench/NotificationPopover.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { ProjectVisibility } from "@/gql/graphql";
import { CakeIcon, Cog8ToothIcon, GlobeAltIcon, LockClosedIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed, watchEffect } from "vue";

const props = defineProps<{ owner: string }>();

const { result: profileResult, loading } = useQuery(
  graphql(/* GraphQL */ `
    query profileHome($slug: String!) {
      ownerBySlug(slug: $slug) {
        ... on User {
          id
          slug
          name
          username
          bot
          description
          createdAt
          canViewDetail
          canWrite
          projects {
            totalCount
            edges {
              node {
                id
                name
                slug
                path
                createdAt
                visibility
                createdAt
                head {
                  name
                  createdAt
                }
              }
            }
          }
        }
        ... on Organization {
          id
          slug
          name
          description
          createdAt
          canViewDetail
          canWrite
          projects {
            totalCount
            edges {
              node {
                id
                name
                slug
                path
                createdAt
                visibility
                createdAt
                head {
                  name
                  createdAt
                }
              }
            }
          }
        }
      }
    }
  `),
  computed(() => ({ slug: props.owner }))
);
const profile = computed(() => profileResult.value?.ownerBySlug);
const user = computed(() => (profile.value?.__typename === "User" ? profile.value : null));
const organization = computed(() => (profile.value?.__typename === "Organization" ? profile.value : null));
const benches = computed(() => {
  return profile.value?.projects.edges.map((e) => e.node);
});

const title = useTitle();
watchEffect(() => {
  if (profile.value == null && loading.value) {
    title.value = props.owner;
  } else if (profile.value != null) {
    title.value = `${props.owner} • ${profile.value.name}`;
  } else {
    // not found
    title.value = "Page not found";
  }
});

const { getTimeFromNowLongString } = useTimeFromNow();
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
      class="mx-auto mt-8 flex w-full max-w-[1000px] flex-grow flex-col px-8 md:gap-8 md:py-4 lg:flex-row lg:items-baseline"
      v-if="profile != null"
    >
      <!-- Profile info -->
      <div class="flex w-80 flex-col gap-2">
        <!-- Name / username -->
        <div class="border-b-2 border-orange-900/[12%] pb-2">
          <h1 class="flex max-w-full flex-row items-center gap-2 text-gray-900">
            <span class="flex flex-row items-center gap-2">
              <span class="truncate text-2xl font-bold">{{ user?.name || organization?.name }}</span>
              <span
                class="rounded-md border border-orange-900/[12%] bg-yellow-100 px-1.5 py-0.5 text-sm font-bold text-yellow-900"
              >
                {{ profile.__typename == "User" ? (user?.bot ? "AI" : "Human") : "Organization" }}
              </span>
            </span>
            <router-link :to="`/settings/${profile.slug}`" v-if="profile.canViewDetail">
              <Cog8ToothIcon class="h-6 w-6 text-gray-400 hover:text-gray-700" />
            </router-link>
          </h1>
          <h2 class="text-xl text-gray-700">
            {{ profile.slug }}
          </h2>
          <p class="mt-2 text-gray-900" v-if="profile.description">
            {{ profile.description }}
          </p>
        </div>
        <!-- Details -->
        <div class="w-full">
          <div class="flex flex-row items-end gap-1 text-sm text-gray-700">
            <CakeIcon class="inline-block h-5 w-5 text-gray-700" />
            <span>{{ getTimeFromNowLongString(profile.createdAt) }}</span>
          </div>
        </div>
        <!-- Organizations / members -->
      </div>
      <!-- Benches / contributions / activity -->
      <div class="mt-8 flex flex-1 flex-grow flex-col md:mt-0">
        <h3 class="text-2xl font-bold text-gray-900">Benches</h3>
        <div class="mt-4 grid w-full grid-cols-1 gap-6 md:grid-cols-2">
          <!-- Empty state -->
          <div v-if="benches?.length == 0">Nothing here yet.</div>
          <!-- Bench card :BenchCard -->
          <router-link
            v-for="project of benches"
            :key="project.id"
            class="group flex h-28 flex-col justify-between rounded-sm border border-orange-900/[12%] bg-white p-3 shadow-sm ring-0 ring-orange-900 ring-opacity-10 transition-colors duration-75 hover:border-orange-600"
            :to="`/${props.owner}/${project.slug}`"
          >
            <div class="flex max-w-full flex-row items-center justify-between gap-2">
              <h3 class="flex max-w-full flex-row items-center font-bold text-gray-900">
                <span class="flex-shrink truncate">{{ project.name }}</span>
                <component
                  :is="project.visibility != ProjectVisibility.Public ? LockClosedIcon : GlobeAltIcon"
                  class="ml-1.5 h-4 w-4 flex-shrink-0 text-gray-700"
                />
              </h3>
            </div>
            <div class="max-w-full truncate text-xs text-gray-500">{{ project.path.replace(".", "/") }}</div>
          </router-link>
        </div>
      </div>
    </main>
    <GenericNotFound v-if="!loading && profile == null" class="flex-grow pb-12" />
    <NotificationArea />
  </div>
</template>
