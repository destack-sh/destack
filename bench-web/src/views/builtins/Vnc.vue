<script lang="ts" setup>
import { log } from "@/utils/log";
import RFB, { type NoVncEvents, type NoVncOptions } from "@novnc/novnc/lib/rfb";
import { onBeforeUnmount, onMounted, ref, watch, type StyleValue } from "vue";

const DEFAULT_RETRY_DURATION = 1000;
const DEFAULT_RETRY_BACKOFF_FACTOR = 1.5;
const MAX_RETRY_DURATION = 10000;

const props = defineProps<{
  url: string;
  style?: StyleValue;
  rfbOptions?: NoVncOptions;
  autoConnect?: boolean;
  debug?: boolean;
  viewOnly?: boolean;
  focusOnClick?: boolean;
  clipViewport?: boolean;
  dragViewport?: boolean;
  scaleViewport?: boolean;
  resizeSession?: boolean;
  showDotCursor?: boolean;
}>();

const emit = defineEmits<{
  connect: [rfb: RFB | null];
  disconnect: [rfb: RFB | null];
  credentialsRequired: [rfb: RFB | null];
  securityFailure: [e: { detail: { status: number; reason: string } }];
  clipboard: [e: { detail: { text: string } }];
  bell: [];
  capabilities: [e: { detail: { capabilities: RFB["capabilities"] } }];
}>();

// state
const rfb = ref<RFB | null>(null);
const eventListeners = ref<{
  -readonly [Event in keyof NoVncEvents]?: (e: any) => void;
}>({});
const attempt = ref<number>(0);
const retryTimeout = ref<any | null>(null);
const isConnected = ref<boolean>(false);
const isLoading = ref<boolean>(true);

// view
const screenRef = ref<HTMLDivElement | null>(null);

/* Watch for URL changes and reconnect */
watch(
  () => props.url,
  (url: string) => {
    connect();
  },
);

/* Connect/disconnect on lifecycle */
onMounted(() => {
  if (props.autoConnect) {
    connect();
  }
});
onBeforeUnmount(() => {
  disconnect();
});

/* Handle connection event */
function onConnect() {
  emit("connect", rfb.value as RFB | null);
  log.debug("vnc.connect");
  focus();
  isLoading.value = false;
  isConnected.value = true;
  attempt.value = 0;
  if (retryTimeout.value != null) {
    clearTimeout(retryTimeout.value);
    retryTimeout.value = null;
  }
}

/* Handle disconnection event */
function onDisconnect() {
  emit("disconnect", rfb.value as RFB | null);
  if (isConnected.value) {
    log.debug("vnc.disconnect");
  }
  isLoading.value = true;
  isConnected.value = false;
  attempt.value++;
  retryTimeout.value = setTimeout(
    connect,
    Math.min(DEFAULT_RETRY_DURATION * DEFAULT_RETRY_BACKOFF_FACTOR ** attempt.value, MAX_RETRY_DURATION),
  );
}

/* Handle credentials required event */
function onCredentialsRequired() {
  emit("credentialsRequired", rfb.value as RFB | null);
  const password = props.rfbOptions?.credentials?.password;
  if (!password) {
    throw new Error("no password for VNC");
  }
  rfb.value?.sendCredentials({ username: "", target: "", password: password });
}

/* Disconnect from VNC */
function disconnect() {
  if (!rfb.value) {
    return;
  }
  try {
    if (retryTimeout.value != null) {
      clearTimeout(retryTimeout.value);
      retryTimeout.value = null;
    }
    (Object.keys(eventListeners.value) as (keyof NoVncEvents)[]).forEach((event) => {
      if (eventListeners.value[event]) {
        rfb.value!.removeEventListener(event, eventListeners.value[event]);
        eventListeners.value[event] = undefined;
      }
    });
    if ((rfb.value as any)._rfbConnectionState != "disconnected") {
      rfb.value.disconnect();
    }
    rfb.value = null;
    isConnected.value = false;
    // ensure disconnect event is fired (event listeners are removed above)
    onDisconnect();
  } catch (err) {
    log.error("vnc.disconnect.error", err);
    rfb.value = null;
    isConnected.value = false;
  }
}

/* Connect to VNC */
function connect() {
  try {
    // disconnect if already connected
    if (isConnected.value && rfb.value != null) {
      disconnect();
    }

    // bail if we don't have a screen
    if (!screenRef.value) {
      throw new Error("no screen for VNC");
    }

    // clear the screen
    screenRef.value.innerHTML = "";

    // create new RFB instance
    rfb.value = new RFB(screenRef.value, props.url, props.rfbOptions);
    rfb.value.viewOnly = props.viewOnly ?? false;
    rfb.value.focusOnClick = true;
    rfb.value.clipViewport = props.clipViewport ?? false;
    rfb.value.dragViewport = props.dragViewport ?? false;
    rfb.value.resizeSession = props.resizeSession ?? false;
    rfb.value.scaleViewport = props.scaleViewport ?? false;
    rfb.value.showDotCursor = props.showDotCursor ?? true;
    rfb.value.background = "";
    rfb.value.qualityLevel = 7;
    rfb.value.compressionLevel = 2;

    // auto-focus on click (doesn't happen by default for some reason even though focusOnClick is true)
    ((rfb.value as any)._canvas as HTMLCanvasElement).addEventListener("mousedown", focus);

    // hook up events
    eventListeners.value.connect = onConnect;
    eventListeners.value.disconnect = onDisconnect;
    eventListeners.value.credentialsrequired = onCredentialsRequired;
    eventListeners.value.securityfailure = (e) => emit("securityFailure", e);
    eventListeners.value.clipboard = (e) => emit("clipboard", e);
    eventListeners.value.bell = () => emit("bell");
    eventListeners.value.capabilities = (e) => emit("capabilities", e);
    (Object.keys(eventListeners.value) as (keyof NoVncEvents)[]).forEach((event) => {
      if (eventListeners.value[event]) {
        rfb.value!.addEventListener(event, eventListeners.value[event]);
      }
    });

    log.debug("vnc.connect.attempt");
  } catch (err) {
    log.error("vnc.connect.error", err);
  }
}

/* Send credentials to VNC server */
function sendCredentials(credentials: Required<NoVncOptions>["credentials"]) {
  rfb.value?.sendCredentials(credentials);
}

/* Send key to VNC server */
function sendKey(keysym: number, code: string, down?: boolean) {
  rfb.value?.sendKey(keysym, code, down);
}

/* Send Ctrl+Alt+Del to VNC server */
function sendCtrlAltDel() {
  rfb.value?.sendCtrlAltDel();
}

/* Shutdown the remote machine */
function sendShutdown() {
  rfb.value?.machineShutdown();
}

/* Reboot the remote machine */
function sendReboot() {
  rfb.value?.machineReboot();
}

/* Reset the remote machine */
function sendReset() {
  rfb.value?.machineReset();
}

/* Send text to clipboard */
function sendPaste(text: string) {
  rfb.value?.clipboardPasteFrom(text);
}

/* Focus the VNC display */
function focus() {
  rfb.value?.focus();
}

/* Blur the VNC display */
function blur() {
  rfb.value?.blur();
}

defineExpose({
  connect,
  disconnect,
  sendCredentials,
  sendKey,
  sendCtrlAltDel,
  sendShutdown,
  sendReboot,
  sendReset,
  sendPaste,
  rfb: rfb,
  eventListeners,
  isConnected,
  isLoading,
  focus,
  blur,
});
</script>
<template>
  <div class="relative" @click="focus">
    <!-- Screen -->
    <div v-show="!isLoading" ref="screenRef" :style="props.style" class="h-full w-full" @click="focus" />
    <!-- Overlay -->
    <div class="absolute left-0 top-0">
      <!-- ... -->
    </div>
  </div>
</template>
