<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
import { SOCIAL_AUTH_PROVIDERS, encodeProviderUrl } from "@/state/auth";
import { useTitle } from "@vueuse/core";

const props = defineProps<{ next?: string }>();
const title = useTitle();
title.value = "Bench - Sign up";
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
          class="mx-2 rounded-sm py-1 px-2 text-sm hover:bg-orange-50"
        >
          Log in
        </router-link>
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">Sign up</h1>
      <p class="mt-4 text-lg text-orange-700">AI is yours for the making.</p>

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
