<script lang="ts" setup>
import { toNodeReference } from "@/system/graph";
import { space } from "@/system/space";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/intrinsics/Bar.vue";
</script>
<template>
  <div class="w-full">
    <Bar class="h-10 w-full border-b border-gray-300" />
    <Windowed v-if="space" :self="toNodeReference(space)" />
  </div>
</template>
