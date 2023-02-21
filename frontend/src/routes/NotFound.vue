<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileMenuButton from "@/components/basic/ProfileMenuButton.vue";
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileMenuButton />
      </template>
    </FatHeader>
    <main class="mx-auto flex w-full max-w-7xl flex-grow flex-col justify-center px-4 sm:px-6 lg:px-8">
      <div class="flex flex-shrink-0 justify-center">
        <a href="/" class="inline-flex">
          <span class="sr-only">Bench</span>
          <!-- <img class="h-12 w-auto" src="/android-chrome-192x192.png" alt="" /> -->
        </a>
      </div>
      <div class="py-16">
        <div class="text-center">
          <p class="text-sm font-semibold uppercase tracking-wide text-orange-600">404 error</p>
          <h1 class="mt-2 text-4xl font-extrabold tracking-tight text-gray-900 sm:text-5xl">Page not found.</h1>
          <p class="mt-2 text-base text-gray-500">Sorry, we couldn’t find the page you’re looking for.</p>
          <div class="mt-6">
            <router-link to="/" class="text-base font-medium text-orange-600 hover:text-orange-500"
              >Go back home<span aria-hidden="true"> &rarr;</span>
            </router-link>
          </div>
        </div>
      </div>
    </main>
    <footer class="mx-auto w-full max-w-7xl flex-shrink-0 px-4 sm:px-6 lg:px-8">
      <nav class="flex justify-center space-x-4">
        <a href="mailto:support@symbolx.co" class="text-sm font-medium text-gray-500 hover:text-gray-600"
          >Contact Support</a
        >
        <span class="inline-block border-l border-gray-300" aria-hidden="true" />
        <a href="https://twitter.co/symbolx" class="text-sm font-medium text-gray-500 hover:text-gray-600">Twitter</a>
      </nav>
    </footer>
  </div>
</template>
