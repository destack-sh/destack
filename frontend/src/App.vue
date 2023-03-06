<script setup lang="ts">
import { provideGlobalAction } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { errorListeners, type Operation } from "@/state/operations";
import { IS_LOCALHOST } from "@/utils/globals";
import { useSystemVersioning } from "@/utils/system";
import { useFullscreen } from "@vueuse/core";
import { onBeforeUnmount, ref, watch, watchEffect } from "vue";
import { RouterView, useRouter } from "vue-router";

// handle errors in operations with notification
const notifications = useNotifications();
function onError(operation: Operation<unknown>, error: unknown) {
  // check for error response
  console.error(`operation ${operation.type} ${operation.id} failed`, error);
  notifications.show({
    type: "operation.fail",
    kind: "error",
    message: "Operation failed",
    description: `Operation ${operation.type} was rejected (id=${operation.id}).`,
  });
}
errorListeners.push(onError);
onBeforeUnmount(() => {
  errorListeners.splice(errorListeners.indexOf(onError), 1);
});

// always track versioning
const { systemInfo, outOfDate } = useSystemVersioning();
const promptedUpdate = ref(false);
watchEffect(() => {
  // don't prompt when developing locally because we hot-reload automatically
  // but the 'version' constant reload requires a vite server restart, which is annoying
  if (outOfDate.value && !IS_LOCALHOST) {
    if (promptedUpdate.value) {
      // already prompted
      return;
    }
    promptedUpdate.value = true;
    // note that there's a special icon for 'system.upgradeAvailable' in NotificationsArea
    notifications.show({
      type: "system.upgradeAvailable",
      kind: "notice",
      message: "Get a better Bench",
      description: `Bench version ${systemInfo.value.version} is now available.`,
      actionText: "Refresh",
      action: () => {
        window.location.reload();
      },
      showTimeMs: 365 * 24 * 60 * 60 * 1000, // 1 year
    });
  }
});

// auto-redirect to complete signup if not completed
const router = useRouter();
const auth = useAuth();

watchEffect(() => {
  if (auth.loggedIn.value && !auth.me.value?.completedSignup) {
    router.push("/signup/complete");
  }
});

// sync fullscreen
const appearance = useAppearance();
const { isFullscreen, enter, exit } = useFullscreen();
watch(
  () => appearance.fullscreen,
  () => {
    if (appearance.fullscreen && !isFullscreen.value) {
      enter().catch(() => (appearance.fullscreen = false));
    } else if (isFullscreen.value) {
      exit();
    }
  }
);
watch(isFullscreen, () => (appearance.fullscreen = isFullscreen.value));

// provide fullscreen enable/disable
provideGlobalAction({
  id: "appearance.toggleFullscreen",
  label: "Toggle Fullscreen",
  shortcuts: ["alt+f"],
  apply: () => {
    appearance.fullscreen = !appearance.fullscreen;
  },
});
</script>

<template>
  <RouterView />
</template>

<style>
@import "@/assets/base.css";
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans&display=swap");

::-webkit-scrollbar {
  width: 12px;
  height: 12px;
}

::-webkit-scrollbar-thumb {
  background-color: rgba(113, 122, 148, 0.4);
  background-clip: padding-box;
  border: 2px solid rgba(0, 0, 0, 0);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background-color: #fdba74;
}

::-webkit-scrollbar-track {
  background-color: transparent;
}

::selection {
  background-color: #fef08a;
}
</style>
