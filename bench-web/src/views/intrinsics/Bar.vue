<script lang="tsx" setup>
import { user } from "@/system/space";
</script>
<template>
  <div class="w-full flex flex-row">
    <!-- Bench info -->
    <div class="bg">Bench</div>
    <!-- Dock -->
    <div class="bg">Dock</div>
    <!-- User -->
    <div class="bg">
      {{ user.auth.isAuthenticated ? user : "Log in" }}
    </div>
  </div>
</template>
