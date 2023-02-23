<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
import { graphql } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";

const title = useTitle();
title.value = "Home";

const { result } = useQuery(
  graphql(/* GraphQL */ `
    query home {
      me {
        slug
        projects {
          id
          name
          slug
          createdAt
          type
          visibility
        }
      }
    }
  `)
);
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
    <div class="mx-auto max-w-[1000px] px-8 py-8">
      <h1 class="text-2xl text-gray-900">My benches</h1>
      <div class="mt-4 grid w-full grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
        <router-link
          v-for="project of result?.me?.projects"
          :key="project.id"
          class="duration-50 group h-20 rounded-md border border-white bg-white p-3 shadow-sm ring-1 ring-orange-900 ring-opacity-5 transition-colors hover:border-orange-600"
          :to="`/${result?.me?.slug}/${project.slug}`"
        >
          <h3 class="font-bold text-gray-900">{{ project.name }}</h3>
        </router-link>
      </div>

      <h1 class="mt-24 text-2xl text-gray-900">Community</h1>
      <div class="mt-4 grid grid-cols-4">
        <div>Coming soon!</div>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
