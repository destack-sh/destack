<script lang="ts" setup>
import BenchIcon from "@/components/basic/BenchIcon.vue";
</script>
<template>
  <div class="group flex flex-shrink-0 items-center">
    <RouterLink to="/" class="group px-[15px] py-2 hover:bg-orange-100">
      <BenchIcon class="h-5 w-5 transform transition-transform duration-150 group-hover:scale-110" />
    </RouterLink>
  </div>
</template>
