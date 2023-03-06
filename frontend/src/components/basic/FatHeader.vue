<script lang="ts" setup></script>
<template>
  <!-- Standard fat header -->
  <!-- Trying border-gray-200 vs border-orange-900 + border-opacity-20 .. not sure yet -->
  <header
    class="static mx-auto flex w-full flex-shrink-0 justify-between gap-8 overflow-y-visible border-b border-orange-900 border-opacity-20 bg-white"
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
