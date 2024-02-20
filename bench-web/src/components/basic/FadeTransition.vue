<script lang="ts" setup></script>
<template>
  <transition
    enter-active-class="transition duration-75 ease-in"
    enter-from-class="transform opacity-0"
    enter-to-class="transform opacity-100"
    leave-active-class="transition duration-75 ease-in"
    leave-from-class="transform opacity-100"
    leave-to-class="transform opacity-0"
  >
    <slot />
  </transition>
</template>
