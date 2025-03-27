<script lang="ts" setup>
import { log } from "@/utils/log";
import RFB, { type NoVncEvents, type NoVncOptions } from "@novnc/novnc/lib/rfb";
import { onBeforeUnmount, onMounted, ref, watch, withDefaults, type StyleValue } from "vue";

const props = withDefaults(
  defineProps<{
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
    background?: string;
    qualityLevel?: number;
    compressionLevel?: number;
  }>(),
  {
    autoConnect: true,
    retryDuration: 3000,
    debug: false,
  },
);

const emit = defineEmits<{
  connect: [rfb?: RFB];
  disconnect: [rfb?: RFB];
  credentialsRequired: [rfb?: RFB];
  securityFailure: [e?: { detail: { status: number; reason: string } }];
  clipboard: [e?: { detail: { text: string } }];
  bell: [];
  desktopName: [e?: { detail: { name: string } }];
  capabilities: [e?: { detail: { capabilities: RFB["capabilities"] } }];
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
  emit("connect", rfb.value ?? undefined);
  log.debug("vnc.connect");
  isLoading.value = false;
}

/* Handle disconnection event */
function onDisconnect() {
  emit("disconnect", rfb.value ?? undefined);
  if (isConnected.value) {
    log.debug("vnc.disconnect");
    timeouts.value.push(setTimeout(connect, props.retryDuration));
  }
  isLoading.value = true;
}

/* Handle credentials required event */
function onCredentialsRequired() {
  emit("credentialsRequired", rfb.value ?? undefined);
  const password = props.rfbOptions?.credentials?.password;
  if (!password) {
    throw new Error("no password for VNC");
  }
  rfb.value?.sendCredentials({ password: password });
}

/* Handle desktop name event */
function onDesktopName(e: { detail: { name: string } }) {
  emit("desktopName", e);
  log.debug(`vnc.desktopName`, e.detail.name);
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
    rfb.value.disconnect();
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
    rfb.value.focusOnClick = props.focusOnClick ?? false;
    rfb.value.clipViewport = props.clipViewport ?? false;
    rfb.value.dragViewport = props.dragViewport ?? false;
    rfb.value.resizeSession = props.resizeSession ?? false;
    rfb.value.scaleViewport = props.scaleViewport ?? false;
    rfb.value.showDotCursor = props.showDotCursor ?? false;
    rfb.value.background = props.background ?? "";
    rfb.value.qualityLevel = props.qualityLevel ?? 6;
    rfb.value.compressionLevel = props.compressionLevel ?? 2;

    // hook up events
    eventListeners.value.connect = onConnect;
    eventListeners.value.disconnect = onDisconnect;
    eventListeners.value.credentialsrequired = onCredentialsRequired;
    eventListeners.value.securityfailure = (e) => emit("securityFailure", e);
    eventListeners.value.clipboard = (e) => emit("clipboard", e);
    eventListeners.value.bell = () => emit("bell");
    eventListeners.value.desktopname = onDesktopName;
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
function sendCredentials(credentials: NoVncOptions["credentials"]) {
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
function clipboardPaste(text: string) {
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
  connected: isConnected,
  sendCredentials,
  sendKey,
  sendCtrlAltDel,
  sendShutdown,
  sendReboot,
  sendReset,
  clipboardPaste,
  rfb: rfb,
  eventListeners: eventListeners,
  focus,
  blur,
});
</script>
<template>
  <div>
    <!-- Screen -->
    <div v-show="!isLoading" ref="screenRef" :style="props.style" class="h-full w-full bg-white"></div>
    <!-- Overlay -->
    <!-- ... -->
    <!-- Loading -->
    <template v-if="isLoading">
      <slot name="loading">
        <div class="flex h-full w-full items-center justify-center bg-white text-lg font-bold text-[#333333]">
          nocheckin: Loading...
        </div>
      </slot>
    </template>
  </div>
</template>
