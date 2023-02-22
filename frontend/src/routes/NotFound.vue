<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileMenuButton from "@/components/basic/ProfileMenuButton.vue";
import { useTitle } from "@vueuse/core";

const title = useTitle();
title.value = "Page not found";
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
    <GenericNotFound />
  </div>
</template>
