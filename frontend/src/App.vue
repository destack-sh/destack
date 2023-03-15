<script setup lang="ts">
import { provideGlobalAction } from "@/state/actions";
import { hostStatementActions } from "@/state/actions/statement";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { errorListeners, type Operation } from "@/state/operations";
import { IS_LOCALHOST } from "@/utils/globals";
import { useSystemVersioning } from "@/utils/system";
import ArrowUpCircleIcon from "@heroicons/vue/24/outline/ArrowUpCircleIcon";
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
    notifications.show({
      type: "system.upgradeAvailable",
      kind: "notice",
      icon: ArrowUpCircleIcon,
      message: "Get a better Bench",
      description: `Bench version ${systemInfo.value.version} is now available.`,
      actionText: "Refresh",
      action: () => {
        window.location.reload();
      },
      showTimeMs: 365 * 24 * 60 * 60 * 1000, // 1 year (user should upgrade)
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

// host all statement actions (bound to root component)
hostStatementActions();
</script>

<template>
  <RouterView />
</template>

<style>
@import "@/assets/base.css";
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans&display=swap");

::-webkit-scrollbar {
  width: 10px;
  height: 10px;
  transition: opacity 0.075s ease-in-out;
}

::-webkit-scrollbar-thumb {
  background-color: rgba(113, 122, 148, 0);
  background-clip: padding-box;
  border: 2px solid rgba(0, 0, 0, 0);
  border-radius: 2px;
  transition: background-color 0.075s ease-in-out;
}

/* TODO @UX: make scrollbar visible when active in container */
::-webkit-scrollbar-thumb:hover {
  background-color: #fdba74;
}

::-webkit-scrollbar:hover {
  opacity: 1;
}

::-webkit-scrollbar-track {
  background-color: transparent;
}

::selection {
  background-color: #fef08a;
}
</style>
