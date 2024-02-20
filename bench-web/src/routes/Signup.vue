<script lang="ts" setup>
import BenchIcon from "@/components/basic/BenchIcon.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import { SOCIAL_AUTH_PROVIDERS, encodeProviderUrl, useAuth } from "@/state/auth";
import { useTitle } from "@vueuse/core";
import { watchEffect } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{ next?: string }>();
const title = useTitle();
title.value = "Bench - Sign up";

// auto-redirect to next (or home) if logged in
const router = useRouter();
const auth = useAuth();
watchEffect(() => {
  if (auth.loggedIn.value) {
    // assume next is on the same host, so trim to get path
    if (props.next) {
      const url = new URL(props.next);
      router.push(url.pathname);
    } else {
      router.push({ name: "Home" });
    }
  }
});
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <router-link
          :to="{ name: 'Login', query: { next } }"
          class="mx-2 rounded-sm px-2 py-1 text-sm hover:bg-orange-100"
        >
          Log in
        </router-link>
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-2">
        <BenchIcon class="h-4 w-4 text-orange-600" />
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">Sign up</h1>
      <p class="mt-4 text-lg text-orange-700">Build that awesome bot.</p>

      <div class="mx-2 mt-10 flex flex-col gap-3 text-sm">
        <a
          :href="provider.enabled ? encodeProviderUrl(provider.url, props.next) : ''"
          v-for="provider in SOCIAL_AUTH_PROVIDERS"
          :key="provider.name"
          class="duration-50 flex flex-row items-center justify-center gap-2 rounded-sm border border-orange-600 p-1.5 shadow-sm transition-colors hover:bg-orange-600 hover:text-white"
          :class="{ 'pointer-events-none opacity-50': !provider.enabled }"
        >
          Continue with {{ provider.name }}
        </a>
      </div>

      <footer class="mt-10 text-xs text-gray-500">
        By creating an account, you agree to SymbolX's Terms of Service and Privacy Policy.
      </footer>
    </div>
    <NotificationArea />
  </div>
</template>
