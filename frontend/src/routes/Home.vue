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
import { PlusIcon } from "@heroicons/vue/24/outline";

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
          }
        }
      }
    }
  `)
);

const benches = computed(() => {
  return myBenchesResult.value?.me?.projects
    .concat(myBenchesResult.value?.me?.organizations.flatMap((org) => org.projects))
    .sort((a, b) => b.createdAt - a.createdAt);
});
const { getTimeFromNowString } = useTimeFromNow();

const anyLoading = computed(() => myBenchesLoading.value);
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
    <div class="mx-auto max-w-[1000px] px-8 py-8" v-show="!anyLoading">
      <h1 class="flex flex-row items-center">
        <span class="text-2xl font-bold text-gray-900">My Benches</span>
        <router-link
          :to="{ name: 'CreateProject' }"
          class="ml-4 flex flex-row items-center rounded-sm text-sm hover:bg-orange-50"
        >
          <PlusIcon class="h-5 w-5 text-orange-600" />
          <span class="text-gray-700">New bench</span>
        </router-link>
      </h1>
      <div class="mt-4 grid w-full grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
        <router-link
          v-for="project of benches"
          :key="project.id"
          class="duration-50 group flex h-28 flex-col justify-between rounded-sm border border-white bg-white p-3 shadow-sm ring-1 ring-orange-900 ring-opacity-5 transition-colors hover:border-orange-600"
          :to="`/${myBenchesResult?.me?.slug}/${project.slug}`"
        >
          <div class="flex flex-row items-center justify-between gap-2">
            <h3 class="font-bold text-gray-900">
              {{ project.name }}
              <span v-if="project.visibility == ProjectVisibility.Private">p</span>
            </h3>
            <span class="text-gray-500">{{ getTimeFromNowString(project.createdAt) }}</span>
          </div>
          <div class="text-xs text-gray-700">{{ project.path.replace(".", "/") }}</div>
        </router-link>
      </div>

      <h1 class="mt-24 text-2xl font-bold text-gray-900">Community</h1>
      <div class="mt-4 grid grid-cols-4">
        <div>Coming soon!</div>
      </div>
    </div>
    <div class="mx-auto max-w-[1000px] px-8 py-8" v-show="anyLoading">
      <!-- Some loading animation -->
    </div>
    <NotificationArea />
  </div>
</template>
