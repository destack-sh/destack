<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed } from "vue";
import { ProjectVisibility } from "@/gql/graphql";
import { GlobeAltIcon, LockClosedIcon, PlusIcon } from "@heroicons/vue/24/outline";

const title = useTitle();
title.value = "Home";

const { result: myBenchesResult, loading: myBenchesLoading } = useQuery(
  graphql(/* GraphQL */ `
    query home {
      me {
        slug
        projects {
          id
          name
          slug
          path
          createdAt
          type
          visibility
          createdAt
        }
        organizations {
          projects {
            id
            name
            path
            slug
            createdAt
            type
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
  `)
);

const benches = computed(() => {
  return myBenchesResult.value?.me?.projects
    .concat(myBenchesResult.value?.me?.organizations.flatMap((org) => org.projects))
    .sort((a, b) => (b.createdAt < a.createdAt ? -1 : 1));
});
</script>
<template>
  <div class="h-full w-full">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <OmniCreate />
        <ProfileButton />
      </template>
    </FatHeader>
    <!-- My Benches -->
    <div class="mx-auto mt-8 max-w-[1000px] px-8 py-4" v-show="!myBenchesLoading">
      <h1 class="flex flex-row items-center">
        <router-link
          :to="`/${myBenchesResult?.me?.slug}`"
          class="text-2xl font-bold text-gray-900 decoration-gray-900 underline-offset-4 hover:underline"
        >
          My Benches
        </router-link>
      </h1>
      <!-- Benches grid -->
      <div class="mt-4 grid w-full grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
        <!-- Empty state -->
        <router-link :to="{ name: 'CreateProject' }" v-if="benches?.length == 0" class="col-span-full text-gray-900">
          <div>Nothing here yet.</div>
          <div class="text-gray text-gray-700">
            <span class="underline decoration-dotted underline-offset-4 hover:decoration-solid">Create a Bench</span> to
            make something.
          </div>
        </router-link>
        <!-- Bench card :BenchCard -->
        <router-link
          v-for="project of benches"
          :key="project.id"
          class="duration-50 group flex h-28 flex-col justify-between rounded-sm border border-white bg-white p-3 shadow-sm ring-1 ring-orange-900 ring-opacity-5 transition-colors hover:border-orange-600"
          :to="`/${myBenchesResult?.me?.slug}/${project.slug}`"
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
    <div class="mx-auto mt-8 max-w-[1000px] px-8 py-4" v-show="!myBenchesLoading">
      <h1 class="text-2xl font-bold text-gray-900">Community</h1>
      <div class="mt-4 grid grid-cols-4 text-gray-900">
        <div>Coming soon!</div>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
