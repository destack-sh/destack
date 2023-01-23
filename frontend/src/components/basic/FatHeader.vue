<script lang="ts" setup></script>
<template>
  <!-- Standard fat header -->
  <header
    class="static mx-auto flex w-full flex-shrink-0 justify-between gap-8 overflow-y-visible border-b border-gray-200 bg-white"
  >
    <!-- Left side -->
    <div class="static flex items-center">
      <slot name="left" />
    </div>
    <div class="flex min-w-fit flex-shrink-0 items-center justify-end">
      <slot name="right" />
    </div>
  </header>
</template>
