<script lang="ts" setup></script>
<template>
  <!-- Standard fat header -->
  <!--
    TODO @Design: revisit gray-200 vs orange-900 + opacity borders & dividers
    Trying border-gray-200 vs border-orange-900 + border-opacity-[12%]
    (all across the codebase, with slightly different % opacity) 
  -->
  <header
    class="static mx-auto flex w-full flex-shrink-0 justify-between gap-8 overflow-y-visible border-b border-orange-900 border-opacity-[16%] bg-white"
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
