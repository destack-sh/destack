<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import { API_BASE_URL } from "@/utils/globals";
import { useTitle } from "@vueuse/core";

const title = useTitle();
title.value = "Bench - Sign up";

const socialAuthProviders = [
  {
    name: "GitHub",
    url: `${API_BASE_URL}/login/github`,
  },
  {
    name: "GitLab",
    url: `${API_BASE_URL}/login/gitlab`,
  },
  {
    name: "Google",
    url: `${API_BASE_URL}/login/google`,
  },
];
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <router-link :to="{ name: 'Login' }" class="mx-2 rounded-sm py-1 px-2 text-sm hover:bg-orange-50">
          Log in
        </router-link>
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">Sign up</h1>
      <p class="mt-4 text-orange-700">AI is yours for the making.</p>

      <div class="mx-2 mt-10 flex flex-col gap-3 text-sm">
        <a
          :href="provider.url"
          v-for="provider in socialAuthProviders"
          :key="provider.name"
          class="duration-50 flex flex-row items-center justify-center gap-2 rounded-sm border border-orange-600 p-1 shadow-sm transition-colors hover:bg-orange-600 hover:text-white"
        >
          Continue with {{ provider.name }}
        </a>
      </div>

      <footer class="mt-10 text-xs text-gray-500">
        By creating an account, you agree to SymbolX's Terms of Service and Privacy Policy.
      </footer>
    </div>
  </div>
</template>
