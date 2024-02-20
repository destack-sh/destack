<script lang="ts" setup>
import BenchIcon from "@/components/basic/BenchIcon.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import { UserStatus } from "@/gql/graphql";
import { useAuth, useRedirectIfNotLoggedIn } from "@/state/auth";
import { useTitle } from "@vueuse/core";
import { computed } from "vue";
import { useRouter } from "vue-router";

defineProps<{ next?: string }>();
const title = useTitle();
title.value = "Bench - Waitlist";

const auth = useAuth();
const router = useRouter();

useRedirectIfNotLoggedIn({ name: "Home" });

const active = computed(() => auth.me.value != null && auth.me.value.status == UserStatus.Active);
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileButton />
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-2">
        <BenchIcon class="h-4 w-4 text-orange-600" />
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">The Waitlist</h1>
      <div class="" v-if="!active">
        <h3 class="mt-6 font-bold">Bench is currently at capacity.</h3>
        <p class="mt-3 text-orange-700">
          Many people want beautiful bots.<br />
          We're excited to let you in soon.<br />
          Our bots will contact your bots.
        </p>
      </div>
      <div v-else>
        <h3 class="mt-6 font-bold">Bench is ready for you.</h3>
        <p class="mt-3 text-orange-700">Thank you for your patience.</p>
        <div class="mt-5">
          <router-link
            to="/new"
            class="w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          >
            Enter &rarr;
          </router-link>
        </div>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
