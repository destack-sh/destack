<script lang="ts" setup>
import { HELPER_VIEW_TYPES, NODE_VIEW_TYPES } from "@/language/const";
import { NodeReferenceData, NodeType, Orientation, UserStatus, ViewType } from "@/proto/wire";
import { toPlainNodeRef, unwrapProtoOneOf } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { assignSpaceInPackage, bench, canvas, space, spaceConnection, spaceGraph } from "@/system/space";
import { user } from "@/system/user";
import { IS_IN_ALT_MODE, fireActionById } from "@/ui/action";
import { makeIcon } from "@/ui/icon";
import { keytrap } from "@/ui/keymap";
import { isDraggingGlobal } from "@/ui/layout";
import { hasActivePopover } from "@/ui/popover";
import { createDesktopDefaultSpace } from "@/ui/space";
import { toaster } from "@/ui/toast";
import { DISCORD_URL } from "@/utils/globals";
import DragOverlay from "@/views/builtins/DragOverlay.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import Omnibar from "@/views/builtins/Omnibar.vue";
import PopoverOverlay from "@/views/builtins/PopoverOverlay.vue";
import ToastOverlay from "@/views/builtins/ToastOverlay.vue";
import TooltipOverlay from "@/views/builtins/TooltipOverlay.vue";
import Split from "@/views/containers/Split.vue";
import Button from "@/views/controls/Button.vue";
import { useTitle, useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";

const spaceRef = ref<HTMLElement | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const mainBox = computed(() => ({ left: 0, top: 0, width: spaceWidth.value, height: spaceHeight.value }));
const windows = spaceGraph.getChildrenRef(spacePtr, NodeType.VIEW);
const window = computed(() => windows.value[0]); // assumes :OneRootWindow for now
const omnibarRef = ref<InstanceType<typeof Omnibar> | null>(null);

// suppress save everywhere
const unbind = keytrap.bind(["ctrl+s", "mod+s"], () => true);
onBeforeUnmount(() => unbind()); // for hot reload

// sync browser title
const viewAncestors = canvas.graph.getAncestorsRef(canvas.focusedViewPtr, {
  metatypes: [NodeType.VIEW],
  includeSelf: true,
});
const viewBase = computed(() =>
  viewAncestors.value.find((v) => NODE_VIEW_TYPES.has(v.type) || HELPER_VIEW_TYPES.has(v.type)),
);
const viewBaseNodePtr: Ref<NodeReferenceData | undefined> = computed(() => unwrapProtoOneOf(viewBase.value?.nodePtr));
const viewBaseNode = canvas.graph.getRef(viewBaseNodePtr);
const browserTitle = useTitle();
watch(
  [bench, viewBase, viewBaseNode],
  () => {
    const benchPostfix = bench.value == null ? "Bench" : bench.value?.slug;
    const viewTitle = (viewBaseNode.value as any)?.name ?? (viewBaseNode.value as any)?.title ?? viewBase.value?.title;
    browserTitle.value = viewTitle ? `${viewTitle} | @${benchPostfix}` : `@${benchPostfix}`;
  },
  { immediate: true },
);
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="select-none overflow-hidden bg-white text-sm"
    :class="[
      isDraggingGlobal || hasActivePopover ? 'pointer-events-none select-none' : '',
      IS_IN_ALT_MODE ? 'altmode' : '',
    ]"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="() => {} /* suppress generic context menu */"
  >
    <!-- Space root (:OneRootWindow) -->
    <Split
      v-if="window"
      id="window"
      class="absolute"
      :type="ViewType.WINDOW"
      :self="toPlainNodeRef(window)"
      :focus="window.focus"
      :name="window.name"
      :size="mainBox"
      :orientation="Orientation.HORIZONTAL"
    />
    <!-- Loading... -->
    <div v-else-if="!spaceConnection.isConnected.value" class="absolute h-full w-full bg-white">
      <Inaccessible class="h-full w-full" :node="spacePtr" :connection="spaceConnection" />
    </div>
    <!-- Does not have a space (not signed, space empty or disappeared) -->
    <div v-else class="absolute flex h-full w-full flex-col justify-center bg-white text-center">
      <div v-if="space && bench" class="flex w-fit flex-col gap-y-2 self-center">
        <!-- Space empty for some reason -->
        <span>
          <i class="fas fa-empty-set mr-1.5 text-gray-500" />
          <span class="text-gray-600">Space Is Empty</span>
        </span>
        <Button
          id="create"
          name="Create"
          :icon="makeIcon('fas fa-redo-alt')"
          title="Restore Default"
          @click="() => createDesktopDefaultSpace(canvas.tx(), space!)"
        />
      </div>
      <div v-else-if="bench" class="flex w-fit flex-col self-center">
        <!-- Space inaccessible for some reason -->
        <span>
          <i class="fas fa-circle-exclamation mr-1.5 text-gray-500" />
          <span class="text-gray-600"
            >Space not found in <span class="font-medium">@{{ bench.slug }}</span></span
          >
        </span>
        <span class="text-gray-400">
          (<span v-if="user"
            >Logged in as <span class="font-medium">{{ user.slug }}</span></span
          >
          <span v-else>Not logged in</span>)
        </span>
        <Button
          v-if="user"
          id="user"
          class="mt-2"
          name="MakeSpace"
          :icon="makeIcon('fas fa-plus')"
          title="Make Space"
          @click="() => assignSpaceInPackage()"
        />
        <Button
          v-else
          id="login"
          class="mt-2"
          name="LogIn"
          :icon="makeIcon('fas fa-arrow-right-to-bracket')"
          title="Log In"
          @click="fireActionById('user.auth.login')"
        />
      </div>
      <div v-else-if="user && user.status == UserStatus.WAITLISTED" class="flex flex-col gap-y-2 self-center">
        <!-- Logged in but waitlisted -->
        <span>
          <i class="fas fa-clock mr-1.5 text-gray-500" />
          <span class="text-gray-600"
            ><span class="font-semibold">{{ user.slug }}</span> is on the waitlist</span
          >
        </span>
        <a :href="DISCORD_URL" class="mt-1 underline">Discord</a>
      </div>
      <div v-else-if="user" class="flex flex-col gap-y-2 self-center">
        <!-- Logged in but not on any space (not sure if this should even show or just auto-redirect?) -->
        <span>
          <i class="fas fa-circle-exclamation mr-1.5 text-gray-500" />
          <span class="text-gray-600">You're Lost in Space</span>
        </span>
        <Button
          v-if="user.status == UserStatus.ACTIVATED"
          id="home"
          name="GoHome"
          :icon="makeIcon('fas fa-home')"
          title="Go Home"
          @click="fireActionById('user.misc.goToHome')"
        />
        <Button
          v-else
          id="activate"
          name="Activate"
          :icon="makeIcon('fas fa-plus')"
          title="Create Bench"
          @click="fireActionById('user.auth.activate')"
        />
      </div>
      <div v-else class="flex flex-col gap-y-2 self-center">
        <!-- Not logged in, not on a space (general landing page should go here) -->
        <h2 class="mb-1.5 text-2xl font-bold">Bench</h2>
        <Button
          id="login"
          name="LogIn"
          :icon="makeIcon('fas fa-arrow-right-to-bracket')"
          title="Log In"
          @click="fireActionById('user.auth.login')"
        />
      </div>
    </div>

    <!-- Overlays -->
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <TooltipOverlay />
    <PopoverOverlay />
    <DragOverlay />
  </div>
</template>
<style>
/* stop overscrolling */
* {
  overscroll-behavior: none;
  scrollbar-gutter: overlay;
}

/* hide scrollbar with .scrollbar-none */
.scrollbar-none {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.scrollbar-none::-webkit-scrollbar {
  display: none;
}

/** make selections match primary color */
::selection {
  background-color: #fcd34d;
}

.caret-transparent {
  caret-color: transparent;
}

/* marks are bold+underline by default */
mark {
  background-color: #fcd34d;
  color: inherit;
  font-weight: bold;
}

.mark-bold mark {
  font-weight: bold;
}

.mark-semibold mark {
  font-weight: 600;
}

.mark-underlined mark {
  text-decoration: underline;
  text-underline-offset: 2px;
}

.mark-primary mark {
  background-color: #fcd34d;
  font-weight: normal;
  text-decoration: none;
}

.mark-secondary mark {
  background-color: #bae6fd;
  font-weight: normal;
  text-decoration: none;
}
</style>
