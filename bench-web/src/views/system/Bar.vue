<script lang="tsx" setup>
import { makeIcon } from "@/system/icon";
import { bench, user } from "@/system/space";
import Button from "@/views/controls/Button.vue";
import Dock from "@/views/system/Dock.vue";
</script>
<template>
  <div class="flex w-full flex-row items-center justify-between gap-x-4 bg-gray-100 px-4 text-sm text-gray-900">
    <!-- Left -->
    <div class="flex flex-shrink-0">
      <!-- Bench -->
      <button class="rounded-md border border-gray-300 bg-white px-2 py-1" v-if="bench">Bench</button>
      <button v-else class="rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300">
        <span class="font-semibold">Bench</span>
        <span class="skew-y-4 inset-thick-underline ml-1 pl-0.5 font-semibold">Beta</span>
      </button>
      <!-- Status -->
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div class="flex flex-1 flex-shrink-0 items-center justify-center -sm:hidden">
      <!-- Dock -->
      <Dock class="w-fit rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300" />
    </div>

    <!-- Right -->
    <div class="flex flex-shrink-0 flex-row">
      <!-- User -->
      <div v-if="user" class="">
        <button>
          {{ user.slug }}
        </button>
      </div>
      <div v-else class="">
        <Button title="Sign in" :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })" @click="() => {}" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.inset-thick-underline {
  position: relative;
  z-index: 2;
  display: inline-block;
}

.inset-thick-underline:after {
  content: "";
  z-index: -1;
  position: absolute;
  width: 100%;
  border-radius: 1px;
  height: 4px;
  background-color: #fcd34d;
  bottom: 2px;
  transform: skewY(-3deg);
  left: 1px;
}
</style>
