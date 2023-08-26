<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import NotificationPopover from "@/components/bench/NotificationPopover.vue";
import { graphql } from "@/gql";
import { ProjectVisibility } from "@/gql/graphql";
import { GlobeAltIcon, LockClosedIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed } from "vue";

const title = useTitle();
title.value = "Home • Bench";

const { result: myBenchesResult, loading: myBenchesLoading } = useQuery(
  graphql(/* GraphQL */ `
    query homeBenches {
      me {
        id
        slug
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
              description
            }
          }
        }
        organizations {
          edges {
            node {
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
                    description
                  }
                }
              }
            }
          }
        }
      }
    }
  `)
);

const { result: communityBenchesResult, loading: communityBenchesLoading } = useQuery(
  graphql(/* GraphQL */ `
    query featuredBenches {
      featuredProjects(last: 5) {
        totalCount
        edges {
          node {
            id
            name
            slug
            path
            createdAt
            visibility
            description
          }
        }
      }
    }
  `)
);

const myBenches = computed(() => {
  return myBenchesResult.value?.me?.projects.edges
    .map((e) => e.node)
    .concat(myBenchesResult.value?.me?.organizations.edges.flatMap((org) => org.node.projects.edges.map((e) => e.node)))
    .sort((a, b) => (b.createdAt < a.createdAt ? -1 : 1));
});
const communityBenches = computed(() => communityBenchesResult.value?.featuredProjects.edges.map((e) => e.node));
</script>
<template>
  <div class="h-full w-full bg-gray-50">
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
        <router-link
          :to="{ name: 'CreateBench' }"
          v-if="myBenches == null || myBenches?.length == 0"
          class="col-span-full text-gray-900"
        >
          <div>Nothing here yet.</div>
          <div class="text-gray text-gray-700">
            <span class="underline decoration-dotted underline-offset-4 hover:decoration-solid">Create a Bench</span> to
            make something.
          </div>
        </router-link>
        <!-- Bench card :BenchCard -->
        <router-link
          v-for="project of myBenches"
          :key="project.id"
          class="group flex h-28 flex-col justify-between rounded-sm border border-orange-900 border-opacity-20 bg-white p-3 shadow-sm ring-0 ring-orange-900 ring-opacity-5 transition-colors duration-75 hover:border-orange-600"
          :to="`/${project.path.replace('.', '/')}`"
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
    <!-- Featured/community work -->
    <div class="mx-auto mt-8 max-w-[1000px] px-8 py-4" v-show="!communityBenchesLoading">
      <h1 class="flex items-center gap-2.5 text-2xl font-bold text-gray-900">
        Community
        <!-- Soon -->
        <span class="rounded-sm border border-orange-600 px-1 text-sm font-bold text-orange-600"> soon </span>
      </h1>
      <!-- Communuity Benches grid -->
      <div class="mt-4 grid w-full grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
        <!-- Bench card :BenchCard -->
        <router-link
          v-for="project of communityBenches"
          :key="project.id"
          class="group flex h-28 flex-col justify-between rounded-sm border border-orange-900 border-opacity-20 bg-white p-3 shadow-sm ring-0 ring-orange-900 ring-opacity-5 transition-colors duration-75 hover:border-orange-600"
          :to="`/${project.path.replace('.', '/')}`"
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
    <NotificationArea />
  </div>
</template>
