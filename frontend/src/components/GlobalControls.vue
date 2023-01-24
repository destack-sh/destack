<script lang="ts" setup>
import { useCurrentModuleRuntime } from "@/state/runtime";
import { CloudArrowUpIcon, ShareIcon } from "@heroicons/vue/24/outline";
import { useClipboard } from "@vueuse/core";
import { computed } from "vue";

const runtime = useCurrentModuleRuntime();
const canDeploy = computed(() => runtime.errors?.value != null && runtime.errors.value.length == 0);

const { copy } = useClipboard();

function share() {
  // should probably open a share & permissions menu
  // but just copy current url to clipboard for now
  copy(window.location.href);
  // should probably give a notification here
  console.log("share");
}

function deploy() {
  // should also open proper menu here
  console.log("deploy");
}
</script>
<template>
  <!-- Share -->
  <button class="rounded-sm p-1 text-sm hover:bg-orange-50" @click="share">
    <ShareIcon class="h-5 w-5 text-gray-900" />
  </button>
  <!-- Deploy -->
  <button
    class="rounded-sm p-1 text-sm"
    :class="{
      'text-green-900 hover:bg-green-50': canDeploy,
      'text-gray-500': !canDeploy,
    }"
    :disabled="!canDeploy"
    @click="deploy"
  >
    <CloudArrowUpIcon class="h-5 w-5" />
  </button>
</template>
