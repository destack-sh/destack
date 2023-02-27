<script lang="ts" setup>
import { provideGlobalAction } from "@/state/actions";
import { useNotifications } from "@/state/notifications";
import { ShareIcon } from "@heroicons/vue/24/outline";
import { useClipboard } from "@vueuse/core";

const { copy } = useClipboard();
const notifications = useNotifications();

const share = provideGlobalAction({
  id: "share.link",
  label: "Share link",
  shortcuts: [],
  apply: () => {
    // should probably open a share & permissions menu
    // but just copy current url to clipboard for now
    copy(window.location.href);
    notifications.show({
      kind: "success",
      type: "share.success",
      message: "Shared",
      description: "Your sharing link is in your clipboard.",
    });
  },
});
</script>
<template>
  <!-- Share -->
  <button class="rounded-sm p-1 text-sm hover:bg-orange-50" @click="share.apply">
    <ShareIcon class="h-5 w-5 text-orange-600" />
  </button>
</template>
