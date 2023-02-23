<script lang="ts" setup>
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
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
    Welcome home
    <NotificationArea />
  </div>
</template>
