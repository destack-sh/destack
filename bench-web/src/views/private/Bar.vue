<script lang="tsx" setup>
import { NodeType, Orientation } from "@/proto/wire";
import { fireActionById } from "@/system/action";
import { isDeveloperMode } from "@/system/client";
import { graphConnections } from "@/system/connection";
import { IconInline, makeIcon } from "@/system/icon";
import { DEFAULT_USER_ICON, ICON_BY_NODE_TYPE } from "@/system/lang";
import { bench, hasLocalBench } from "@/system/space";
import { client, clientsSorted, isAuthenticated, user } from "@/system/user";
import { COMMIT, IS_DEBUG, VERSION } from "@/utils/globals";
import { ScrollbarWidth, isDraggingGlobal } from "@/utils/layout";
import { menuActionsLike, menuItemFromAction } from "@/utils/menu";
import { humanizeBytes } from "@/utils/string";
import { formatDurationFromNow } from "@/utils/time";
import type { TooltipInfo } from "@/utils/tooltip";
import Scroll from "@/views/containers/Scroll.vue";
import Button from "@/views/controls/Button.vue";
import Dock from "@/views/private/Dock.vue";
import Menu from "@/views/private/Menu.vue";
import Popover from "@/views/private/Popover.vue";
import { useElementSize, useFps, useMemory } from "@vueuse/core";
import { computed, ref, watchEffect } from "vue";

const props = defineProps<{
  box: { x: number; y: number; width: number; height: number };
}>();

const middleRef = ref<HTMLElement | null>(null);
const middleSize = useElementSize(middleRef);
const middlePosition = computed(() => ({
  x: props.box.x + props.box.width / 2 - middleSize.width.value / 2,
  y: props.box.y + props.box.height / 2 - middleSize.height.value / 2,
}));

const fps = useFps({ every: 15 });
const memory = useMemory();

const BENCH_MENU_ITEMS = computed(() => {
  const items = [
    // bench
    menuItemFromAction("bench.go.goToBench", { category: "bench" }),
    menuItemFromAction("bench.go.goToEnvironment", { category: "bench" }),
    menuItemFromAction("bench.go.goToBranch", { category: "bench" }),
    menuItemFromAction("bench.go.goToPackage", { category: "bench" }),
    menuItemFromAction("bench.go.goToSpace", { category: "bench" }),
    // main
    {
      id: "omnibar",
      category: "main",
      icon: "fas fa-magnifying-glass",
      title: "Search",
      action: { items: menuActionsLike({ prefix: ["space.omnibar"] }) },
    },
    {
      id: "view",
      category: "main",
      icon: ICON_BY_NODE_TYPE[NodeType.VIEW],
      title: "View",
      action: { items: menuActionsLike({ prefix: ["view"] }) },
    },
    {
      id: "edit",
      category: "main",
      icon: "fas fa-hammer",
      title: "Edit",
      action: { items: menuActionsLike({ prefix: ["common.edit", "common.move", "common.search"] }) },
    },
    {
      id: "sense",
      category: "main",
      icon: "fas fa-telescope",
      title: "Analyze",
      action: {
        items: [menuItemFromAction("space.launch.inspector"), ...menuActionsLike({ prefix: ["common.sense"] })],
      },
    },
    {
      id: "session",
      category: "main",
      icon: "fas fa-play",
      title: "Run",
      action: { items: menuActionsLike({ prefix: ["common.session"] }) },
    },
    // extra
    menuItemFromAction("space.launch.explorer"),
    menuItemFromAction("space.launch.outline"),
    menuItemFromAction("space.launch.library"),
    menuItemFromAction("space.launch.docs"),
    menuItemFromAction("space.launch.discord"),
  ];

  if (isDeveloperMode.value) {
    items.push({
      id: "developer",
      category: "developer",
      icon: "fas fa-bug",
      title: "Developer",
      action: { items: menuActionsLike({ prefix: ["developer"] }) },
    });
  }

  return items;
});

const USER_MENU_ITEMS = computed(() => {
  const items = [
    menuItemFromAction("user.misc.goToHome", { category: "primary" }),
    menuItemFromAction("space.launch.notifications", { category: "primary" }),
  ];
  if (isAuthenticated.value && !hasLocalBench.value) {
    items.push(menuItemFromAction("user.auth.activate", { category: "primary" }));
  }
  items.push(
    ...[
      menuItemFromAction("user.settings.editKeybindings", { category: "primary" }),
      menuItemFromAction("user.auth.logout", { category: "secondary" }),
    ],
  );
  return items;
});
</script>
<template>
  <div
    class="flex w-full flex-row items-center justify-between gap-x-4 px-4 text-sm text-gray-900"
    data-outside-view="true"
  >
    <!-- Left -->
    <div class="flex flex-shrink-0 flex-row items-center gap-x-4">
      <!-- Bench -->
      <Popover placement="bottom-left" :reference-margin="4" :container-margin="4">
        <template v-slot:trigger="{ toggle, isOpen }">
          <button
            class="flex select-none flex-row items-center rounded-md border px-2 py-1 text-gray-900 shadow-sm shadow-gray-300 hover:cursor-pointer hover:border-gray-400 hover:bg-gray-100"
            :class="[isOpen ? 'border-gray-400 bg-gray-100' : 'border-gray-300 bg-white']"
            @click="toggle"
          >
            <div class="mr-2 h-5 w-6 rounded-md border border-gray-300 bg-primary-300 px-0.5"></div>
            <template v-if="bench">
              <span class="select-all font-semibold">{{ bench.slug }}</span>
            </template>
            <template v-else>
              <span class="select-none font-semibold">Bench</span>
              <span class="ml-1 select-none pl-0.5 font-semibold italic underline decoration-primary-400 decoration-2"
                >Beta</span
              >
            </template>
          </button>
        </template>

        <template v-slot:content="{ close }">
          <!-- Bench Menu -->
          <Menu @close="close" :items="BENCH_MENU_ITEMS">
            <!-- Bench Info -->
            <template v-if="bench" #header>
              <div class="flex flex-row px-2.5 pb-2 pt-1.5">
                <div class="mr-2 w-10 rounded-md border border-gray-700 bg-primary-300 py-0.5 text-center text-lg">
                  <IconInline v-if="bench.icon" class="" v-bind="bench.icon" />
                </div>
                <div class="flex flex-col leading-tight">
                  <span class="select-all font-medium">{{ bench?.name ?? "???" }}</span>
                  <span class="select-all text-gray-500">{{ bench?.slug ?? "???" }}</span>
                </div>
              </div>
            </template>
            <!-- Build Info -->
            <template #footer>
              <div class="flex w-full flex-row px-2.5 pt-2 text-gray-500">
                <span class="select-all">Bench Web</span>
                <span class="ml-auto select-all">{{ VERSION }}</span>
              </div>
              <div class="flex w-full flex-row px-2.5 pb-1.5 text-xs text-gray-500">
                <span class="select-all">{{ IS_DEBUG ? "developmnet" : "production" }}</span>
                <span class="ml-auto select-all">#{{ COMMIT?.slice(0, 8) ?? "???" }}</span>
              </div>
            </template>
          </Menu>
        </template>
      </Popover>

      <!-- Status -->
      <div class="flex flex-row items-center gap-x-3">
        <!-- Connection -->
        <Popover placement="bottom" :reference-margin="8" :container-margin="4">
          <template #trigger="{ toggle }">
            <button
              class="select-none rounded-md border-2 px-1 py-0.5 transition-colors"
              :class="
                graphConnections.some((c) => c.isPaused.value || c.txBuffer.isPaused.value)
                  ? 'border-secondary-600'
                  : 'border-transparent'
              "
              :disabled="!isDeveloperMode"
              @click.stop="toggle"
            >
              <i
                class="fas"
                :class="
                  graphConnections.every((c) => c.isConnected.value)
                    ? 'fa-cloud text-success-700 hover:text-success-800'
                    : 'fa-cloud-slash text-warning-600 hover:text-warning-700'
                "
              />
            </button>
          </template>
          <template #content="{ close }">
            <!-- Connection summary -->
            <!-- will probably move this to a Connections View (maybe keep summary on hover) -->
            <div
              class="p z-50 rounded-md border border-gray-700 bg-white text-gray-900 shadow-md shadow-gray-700"
              v-outside.click.stop="close"
            >
              <div class="my-1 border-b border-gray-700 px-3 py-1">
                <span class="font-semibold">Graph Connections ({{ graphConnections.length }})</span>
              </div>
              <Scroll
                :size="{ width: 400, height: 400 }"
                size-is-dynamic
                :orientation="Orientation.VERTICAL"
                :track-width="ScrollbarWidth.sm"
              >
                <ul class="my-1.5 flex min-w-[320px] flex-col gap-y-1 px-3">
                  <li v-for="connection in graphConnections" :key="connection.id" class="flex flex-row py-0.5">
                    <!-- Metadata -->
                    <span class="rounded-md bg-secondary-100 px-2 font-mono uppercase text-secondary-900">
                      {{ connection.kind }}
                    </span>
                    <span class="ml-2 font-semibold">{{ connection.name }}</span>
                    <span class="ml-2 text-gray-500">#{{ connection.id }}</span>
                    <!-- Status -->
                    <span class="ml-auto flex flex-row pl-4">
                      <span class="mr-2" :class="connection.referenceCount > 0 ? '' : 'text-gray-500'">
                        {{ connection.referenceCount }}
                      </span>
                      <!-- Connected (status) -->
                      <span class="rounded-md px-1 py-0.5">
                        <i
                          class="fas"
                          :class="
                            connection.isConnected.value
                              ? 'fa-check text-success-600'
                              : 'fa-exclamation-circle text-warning-600'
                          "
                        />
                      </span>
                      <!-- Down (status & toggle) -->
                      <button class="rounded-md px-1 py-0.5 hover:bg-primary-200" @click="connection.togglePaused()">
                        <i
                          :class="
                            connection.isFetching.value
                              ? 'fas fa-spinner-third animate-spin text-gray-500'
                              : connection.isLive && !connection.isPaused.value
                                ? 'fas fa-down text-success-600'
                                : 'fas fa-down text-secondary-500'
                          "
                        />
                      </button>
                      <!-- Up (toggle) -->
                      <button
                        class="rounded-md px-1 py-0.5 hover:bg-primary-200"
                        @click="connection.txBuffer.togglePaused()"
                      >
                        <i
                          class="fas fa-up"
                          :class="connection.txBuffer.isPaused.value ? 'text-secondary-500' : 'text-success-600'"
                        />
                      </button>
                    </span>
                  </li>
                </ul>
              </Scroll>
            </div>
          </template>
        </Popover>

        <!-- Developer mode -->
        <div v-if="isDeveloperMode">
          <span
            class="select-none text-hint-700"
            v-tooltip="{icon: 'fas fa-bug', title: 'Developer Mode Enabled'} as TooltipInfo"
          >
            <button class="hover:text-hint-800" @click="fireActionById('developer.misc.toggleDeveloperMode')">
              <i class="fas fa-bug" />
            </button>
            <span class="ml-1">{{ fps }}fps</span>
            <span v-if="memory.isSupported.value && memory.memory.value?.usedJSHeapSize" class="ml-1">
              {{ humanizeBytes(memory.memory.value?.usedJSHeapSize, { cutoff: 1000 }) }}
            </span>
            <span v-if="isDraggingGlobal" class="ml-1"><i class="fas fa-droplet" /></span>
          </span>
        </div>
      </div>
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div
      ref="middleRef"
      class="absolute flex flex-1 flex-shrink-0 items-center justify-center -sm:hidden"
      :style="{ left: middlePosition.x + 'px', top: middlePosition.y + 'px' }"
    >
      <!-- Dock -->
      <Dock class="w-fit px-2 py-1" />
    </div>

    <!-- Right -->
    <div class="flex flex-shrink-0 flex-row">
      <!-- User Menu -->
      <template v-if="user">
        <!-- User (logged in) -->
        <Popover placement="bottom-right" :reference-margin="4" :container-margin="4">
          <template v-slot:trigger="{ toggle, isOpen }">
            <button
              class="flex flex-row items-center rounded-md border px-2 py-1 text-gray-900 shadow-sm shadow-gray-300 hover:cursor-pointer hover:border-gray-400 hover:bg-gray-100"
              :class="[isOpen ? 'border-gray-400 bg-gray-100' : 'border-gray-300 bg-white']"
              @click="toggle"
            >
              <span class="mr-2 rounded-md border border-gray-300 bg-primary-300 px-0.5">
                <IconInline class="" v-bind="user.icon ?? DEFAULT_USER_ICON" />
              </span>
              <span>{{ user.name ?? user.slug }}</span>
            </button>
          </template>
          <template v-slot:content="{ close }">
            <Menu @close="close" :items="USER_MENU_ITEMS">
              <!-- User Info -->
              <template #header>
                <div class="flex flex-row px-2.5 pb-2 pt-1.5">
                  <div class="mr-2 rounded-md border border-gray-700 bg-primary-300 px-2.5 py-0.5 text-xl">
                    <IconInline class="" v-bind="user.icon ?? DEFAULT_USER_ICON" />
                  </div>
                  <div class="flex flex-col leading-tight">
                    <span class="select-all font-medium">{{ user.name }}</span>
                    <span class="select-all text-gray-500">{{ user.slug }}</span>
                  </div>
                </div>
              </template>
              <!-- Client Info -->
              <template #footer>
                <ul class="px-2.5 pb-1.5 pt-2">
                  <li
                    v-for="c in clientsSorted"
                    :key="c.id"
                    class="flex flex-row py-0.5"
                    :class="c.id == client?.id ? 'text-gray-900' : 'text-gray-500'"
                  >
                    <span class="select-all">{{ c.operatingSystem }} - {{ c.browserName }}</span>
                    <span class="ml-auto">
                      <span v-if="c.id == client?.id">current</span>
                      <span v-else-if="c.lastSeenAt">{{
                        formatDurationFromNow(c.lastSeenAt, { format: "approximate" })
                      }}</span>
                    </span>
                  </li>
                </ul>
              </template>
            </Menu>
          </template>
        </Popover>
      </template>
      <!-- User (unauthenticated) -->
      <template v-else>
        <Button
          title="Log In"
          :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })"
          @click="() => fireActionById('user.auth.login')"
        />
      </template>
    </div>
  </div>
</template>
