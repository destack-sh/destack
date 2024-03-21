<script lang="tsx" setup>
import { runAction } from "@/system/action";
import type { GraphConnection } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { bench } from "@/system/space";
import { user } from "@/system/user";
import Button from "@/views/controls/Button.vue";
import Dock from "@/views/private/Dock.vue";

const props = defineProps<{
  spaceConnection: GraphConnection;
}>();
</script>
<template>
  <div
    class="flex w-full flex-row items-center justify-between gap-x-4 bg-gray-100 px-4 text-sm text-gray-900"
    data-outside-view="true"
  >
    <!-- Left -->
    <div class="flex flex-shrink-0">
      <!-- Bench -->
      <button class="rounded-md border border-gray-300 bg-white px-2 py-1" v-if="bench">Bench</button>
      <div v-else class="select-none rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300">
        <span class="font-semibold">Bench</span>
        <span class="ml-1 pl-0.5 font-semibold underline decoration-primary-400 decoration-2">Beta</span>
      </div>
      <!-- Status -->
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div class="flex flex-1 flex-shrink-0 items-center justify-center -sm:hidden">
      <!-- Dock -->
      <Dock class="w-fit px-2 py-1" />
    </div>

    <!-- Right -->
    <div class="flex flex-shrink-0 flex-row">
      <!-- User -->
      <template v-if="user">
        <button class="rounded-md border border-gray-300 bg-white px-2 py-1 font-medium shadow-sm shadow-gray-300">
          {{ user.slug }}
        </button>
      </template>
      <template v-else>
        <Button
          title="Log In"
          :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })"
          @click="() => runAction('user.login')"
        />
      </template>
    </div>
  </div>
</template>
