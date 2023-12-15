<script setup lang="ts">
import { provideGlobalAction } from "@/state/actions";
import { hostStatementActions } from "@/state/actions/statement";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { errorListeners, type Operation } from "@/state/operations";
import { useClient } from "@/state/client";
import { IS_LOCALHOST } from "@/utils/globals";
import { useSystemVersioning } from "@/utils/system";
import ArrowUpCircleIcon from "@heroicons/vue/24/solid/ArrowUpCircleIcon";
import { useFullscreen } from "@vueuse/core";
import { onBeforeUnmount, ref, watch, watchEffect } from "vue";
import { RouterView, useRouter } from "vue-router";
import { OperationMessageKind, type OperationInfo, UserStatus } from "@/gql/graphql";

// handle errors in operations with notification
const notifications = useNotifications();
function onError(operation: Operation<unknown>, error: unknown) {
  // check for error response
  console.error(`operation ${operation.type} ${operation.id} failed`, error);
  let message = "Operation failed";
  let description;
  if ((error as OperationInfo)?.__typename == "OperationInfo") {
    const operror = error as OperationInfo;
    if (operror.messages.some((m) => m.kind == OperationMessageKind.Permission)) {
      message = "Permission denied";
      description = "The bots won't let you do this.";
    } else if (operror.messages.some((m) => m.kind == OperationMessageKind.Validation)) {
      message = "Validation error";
      description = "The bots don't like this request.";
    } else {
      message = "Internal error";
      description = "The bots are confused.";
    }
  } else {
    message = "Unknown error";
    description = `${operation.type} failed (id=${operation.id}).`;
  }
  notifications.show({
    type: "operation.fail",
    kind: "error",
    message,
    description,
  });
}
errorListeners.push(onError);
onBeforeUnmount(() => {
  errorListeners.splice(errorListeners.indexOf(onError), 1);
});

// track client info
const { info: clientInfo, close: closeClient } = useClient();
console.info(`Client: ${clientInfo.value}`);
window.addEventListener("beforeunload", async () => {
  await closeClient();
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
      kind: "success",
      icon: ArrowUpCircleIcon,
      message: "Upgrade your Bench",
      description: `v${systemInfo.value?.version} is ready.`,
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
  if (auth.loggedIn.value && auth.me.value?.status == UserStatus.InitiatedSignup) {
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

// show permanent notification if run on non-Chromium browsers (ugh, will be fixed soon)
if (!(window as any).chrome) {
  notifications.show({
    kind: "error",
    type: "browser.bad",
    message: "Incompatible browser",
    description: "Bench is optimized for Chrome.",
    showTimeMs: 60000,
  });
}

// host all statement actions (bound to root component)
hostStatementActions();
</script>

<template>
  <RouterView />
</template>

<style>
@import "@/assets/base.css";
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans&display=swap");

body {
  /* not wanted anywhere */
  overscroll-behavior: none;
}

::-webkit-scrollbar {
  width: 10px;
  height: 10px;
  transition: all 0.15s ease-in-out;
}

::-webkit-scrollbar-thumb {
  background-color: rgba(113, 122, 148, 0);
  background-clip: padding-box;
  border: 2px solid rgba(0, 0, 0, 0);
  border-radius: 2px;
  transition: all 0.15s ease-in-out;
  opacity: 1;
}

/* turns out visible on hover is annoying, especially without transition */
/* (causes flickering, e.g. for horizontal scrollbars in databases) */
/* ::-webkit-scrollbar-thumb:hover {
  background-color: #fdba74;
} */

/* TODO @UX: tranistion scrollbar properly on active */
.scroll-active::-webkit-scrollbar-thumb {
  background-color: #fdba74;
}

::selection {
  background-color: #fef08a;
}

.scroll-hidden {
  /* Hide the scrollbar */
  scrollbar-width: none; /* For Firefox */
  -ms-overflow-style: none; /* For Internet Explorer and Microsoft Edge legacy */
}
.scroll-hidden::-webkit-scrollbar {
  /* For Chrome, Safari, and Opera */
  display: none;
}
</style>
