<script setup lang="ts">
import { useAuth } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { errorListeners, type Operation } from "@/state/operations";
import { useSystemVersioning } from "@/utils/system";
import { onBeforeUnmount, ref, watchEffect } from "vue";
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
  if (outOfDate.value) {
    if (promptedUpdate.value) {
      // already prompted
      return;
    }
    promptedUpdate.value = true;
    notifications.show({
      type: "system.outOfDate",
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
