<script lang="ts" setup></script>
<template>
  <!-- Standard fat header -->
  <!--
    TODO @Design: revisit gray-200 vs orange-900 + opacity borders & dividers
    Trying border-gray-200 vs border-orange-900 + border-opacity-[12%]
    (all across the codebase, with slightly different % opacity) 
  -->
  <!-- 36 == :GlobalHeaderHeight -->
  <header
    class="static mx-auto flex w-full grid-cols-3 flex-row justify-between gap-8 overflow-y-visible border-b border-orange-900/[12%] bg-white 2xl:grid"
  >
    <!-- Left side -->
    <div class="static flex items-center">
      <slot name="left" />
    </div>
    <!-- Center -->
    <!-- TODO @UX: prevent center from overlapping with left & right -->
    <div class="self-middle flex h-full items-center justify-self-center">
      <slot name="center" />
    </div>
    <!-- Right side -->
    <div class="self-middle flex min-w-fit flex-shrink-0 items-center justify-self-end">
      <slot name="right" />
    </div>
  </header>
</template>
