<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileMenuButton from "@/components/basic/ProfileMenuButton.vue";
</script>
<template>
  <div class="h-full w-full">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileMenuButton />
      </template>
    </FatHeader>
    Welcome home
  </div>
</template>
