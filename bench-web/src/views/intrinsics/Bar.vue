<script lang="tsx" setup>
import userState from "@/system/user";
</script>
<template>
  <div class="w-full flex flex-row">
    <!-- Bench info -->
    <div class="bg">Bench</div>
    <!-- Dock -->
    <div class="bg">Dock</div>
    <!-- User -->
    <div class="bg">
      {{ userState.auth.isAuthenticated ? userState.user.value?.name : "Log in" }}
    </div>
  </div>
</template>
