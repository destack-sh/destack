<script lang="ts" setup>
// just a colored block of a given size
defineProps<{ width: number; height: number }>();
</script>
<template>
  <div :style="{ width: width + 'px', height: height + 'px' }" />
</template>
