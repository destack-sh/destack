<script lang="ts" setup>
import { log } from "@/utils/log";
import RFB, { type NoVncEvents, type NoVncOptions } from "@novnc/novnc/lib/rfb";
import { onBeforeUnmount, onMounted, ref, watch, type StyleValue } from "vue";

const DEFAULT_RETRY_DURATION = 3000;

const props = defineProps<{
  url: string;
  style?: StyleValue;
  rfbOptions?: NoVncOptions;
  autoConnect?: boolean;
  retryDuration?: number;
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

const rfb = ref<RFB | null>(null);
const eventListeners = ref<{
  -readonly [Event in keyof NoVncEvents]?: (e: any) => void;
}>({});
const screenRef = ref<HTMLDivElement | null>(null);
const timeouts = ref<Array<any>>([]);
const isConnected = ref<boolean>(false);
const isLoading = ref<boolean>(true);

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
}

/* Handle disconnection event */
function onDisconnect() {
  emit("disconnect", rfb.value as RFB | null);
  if (isConnected.value) {
    log.debug("vnc.disconnect");
    timeouts.value.push(setTimeout(connect, props.retryDuration ?? DEFAULT_RETRY_DURATION));
  }
  isLoading.value = true;
}

/* Handle credentials required event */
function onCredentialsRequired() {
  emit("credentialsRequired", rfb.value as RFB | null);
  const password = props.rfbOptions?.credentials?.password;
  if (!password) {
    throw new Error("no password for VNC");
  }
  rfb.value?.sendCredentials({ password: password });
}

/* Disconnect from VNC */
function disconnect() {
  if (!rfb.value) {
    return;
  }
  try {
    timeouts.value.forEach(clearTimeout);
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

    isConnected.value = true;
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
  console.log("focus");
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
  <div class="relative h-full w-full p-3" @click="focus">
    <!-- Screen -->
    <div v-show="!isLoading" ref="screenRef" :style="props.style" class="h-full w-full" @click="focus" />
    <!-- Overlay -->
    <div class="absolute left-0 top-0">
      <!-- ... -->
    </div>
    <!-- Loading -->
    <Transition
      enter-from-class="opacity-0"
      enter-active-class="transition-opacity duration-200"
      enter-to-class="opacity-100"
      appear
      mode="out-in"
    >
      <div v-if="isLoading" class="flex h-full w-full items-center justify-center text-lg font-bold">
        <i class="fas fa-spinner-third animate-spin text-gray-400" />
      </div>
    </Transition>
  </div>
</template>
