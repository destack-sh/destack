<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import { getActionsLike, runAction } from "@/system/action";
import type { GraphConnection } from "@/system/connection";
import { IconInline, makeIcon } from "@/system/icon";
import { DEFAULT_BENCH_ICON, DEFAULT_USER_ICON, ICON_BY_NODE_TYPE } from "@/system/lang";
import { clientMeta, isDeveloperMode } from "@/system/local";
import { bench } from "@/system/space";
import { client, user } from "@/system/user";
import { useElementSize } from "@/utils/element";
import { COMMIT, VERSION } from "@/utils/globals";
import { menuItemFromAction } from "@/utils/menu";
import Button from "@/views/controls/Button.vue";
import Dock from "@/views/private/Dock.vue";
import Menu from "@/views/private/Menu.vue";
import Popover from "@/views/private/Popover.vue";
import { computed, ref } from "vue";

const props = defineProps<{
  spaceConnection: GraphConnection;
  box: { x: number; y: number; width: number; height: number };
}>();

const middleRef = ref<HTMLElement | null>(null);
const middleSize = useElementSize(middleRef);
const middlePosition = computed(() => ({
  x: props.box.x + props.box.width / 2 - middleSize.width.value / 2,
  y: props.box.y + props.box.height / 2 - middleSize.height.value / 2,
}));

const BENCH_MENU_ITEMS = computed(() => {
  const items = [
    // bench
    menuItemFromAction("bench.goToBench", { category: "bench" }),
    menuItemFromAction("bench.goToEnvironment", { category: "bench" }),
    menuItemFromAction("bench.goToBranch", { category: "bench" }),
    menuItemFromAction("bench.goToPackage", { category: "bench" }),
    menuItemFromAction("bench.goToSpace", { category: "bench" }),
    // main
    {
      id: "omnibar",
      category: "main",
      icon: "fas fa-magnifying-glass",
      title: "Search",
      action: {
        items: [...getActionsLike({ prefix: "space.omnibar" })].map((action) => menuItemFromAction(action)),
      },
    },
    {
      id: "space",
      category: "main",
      icon: ICON_BY_NODE_TYPE[NodeType.SPACE],
      title: "Space",
      action: {
        items: [...getActionsLike({ prefix: "space.launch" })].map((action) => menuItemFromAction(action)),
      },
    },
    {
      id: "view",
      category: "main",
      icon: ICON_BY_NODE_TYPE[NodeType.VIEW],
      title: "View",
      action: {
        items: getActionsLike({ prefix: "view" }).map((action) => menuItemFromAction(action)),
      },
    },
    {
      id: "edit",
      category: "main",
      icon: "fas fa-hammer",
      title: "Edit",
      action: {
        items: ["common.edit", "common.move", "common.select"]
          .flatMap((prefix) => getActionsLike({ prefix }))
          .map((action) => menuItemFromAction(action)),
      },
    },
    {
      id: "sense",
      category: "main",
      icon: "fas fa-telescope",
      title: "Analyze",
      action: {
        items: [...getActionsLike({ prefix: "common.sense" })].map((action) => menuItemFromAction(action)),
      },
    },
    {
      id: "session",
      category: "main",
      icon: "fas fa-play",
      title: "Run",
      action: {
        items: getActionsLike({ prefix: "common.session" }).map((action) => menuItemFromAction(action)),
      },
    },
  ];

  if (isDeveloperMode.value) {
    items.push({
      id: "developer",
      category: "developer",
      icon: "fas fa-bug",
      title: "Developer",
      action: {
        items: getActionsLike({ prefix: "developer" }).map((action) => menuItemFromAction(action)),
      },
    });
  }

  return items;
});

const USER_MENU_ITEMS = computed(() => [
  menuItemFromAction("user.goToHome", { category: "primary" }),
  menuItemFromAction("user.activate", { category: "primary" }),
  menuItemFromAction("space.launch.notifications", { category: "primary" }),
  menuItemFromAction("user.editKeybindings", { category: "secondary" }),
  menuItemFromAction("user.logout", { category: "secondary" }),
]);
</script>
<template>
  <div
    class="flex w-full flex-row items-center justify-between gap-x-4 bg-gray-100 px-4 text-sm text-gray-900"
    data-outside-view="true"
  >
    <!-- Left -->
    <div class="flex flex-shrink-0 flex-row items-center gap-x-4">
      <!-- Bench -->
      <Popover placement="bottom-left" :reference-margin="4">
        <template v-slot:trigger="{ toggle }">
          <button
            class="flex flex-row items-center rounded-md border border-gray-300 bg-white px-2 py-1 text-gray-900 shadow-sm shadow-gray-300 hover:cursor-pointer hover:border-gray-400 hover:bg-gray-100"
            @click="toggle"
          >
            <div class="mr-2 h-5 w-6 rounded-md border border-gray-300 bg-primary-300 px-0.5"></div>
            <span class="font-semibold">Bench</span>
            <span class="ml-1 pl-0.5 font-semibold underline decoration-primary-400 decoration-2">Beta</span>
          </button>
        </template>
        <template v-slot:content="{ close }">
          <Menu @close="close" :items="BENCH_MENU_ITEMS">
            <!-- Bench Info -->
            <template v-if="bench" #header>
              <div class="flex flex-row px-2.5 pb-2 pt-1.5">
                <div class="mr-2 w-10 rounded-md border border-gray-700 bg-primary-300 py-0.5 text-center text-lg">
                  <IconInline v-if="bench?.icon" class="" v-bind="bench.icon" />
                </div>
                <div class="flex flex-col leading-tight">
                  <span class="font-medium">{{ bench?.name ?? "???" }}</span>
                  <span class="text-gray-500">{{ bench?.slug ?? "???" }}</span>
                </div>
              </div>
            </template>
            <!-- Build Info -->
            <template #footer>
              <div class="flex w-full flex-row px-2.5 pb-1.5 pt-2 text-gray-500">
                <span>Bench Web {{ VERSION }}</span>
                <span class="ml-auto">{{ COMMIT }}</span>
              </div>
            </template>
          </Menu>
        </template>
      </Popover>
      <!-- Status -->
      <div>
        <!-- Connection -->
        <span class="select-none text-success-600">
          <i class="fas fa-wifi mr-1.5" />
        </span>
      </div>
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div
      ref="middleRef"
      class="absolute flex flex-1 flex-shrink-0 items-center justify-center bg-gray-100 -sm:hidden"
      :style="{ left: middlePosition.x + 'px', top: middlePosition.y + 'px' }"
    >
      <!-- Dock -->
      <Dock class="w-fit px-2 py-1" />
    </div>

    <!-- Right -->
    <div class="flex flex-shrink-0 flex-row">
      <template v-if="user">
        <!-- User (logged in) -->
        <Popover placement="bottom-right" :reference-margin="4">
          <template v-slot:trigger="{ toggle }">
            <button
              class="flex flex-row items-center rounded-md border border-gray-300 bg-white px-2 py-1 text-gray-900 shadow-sm shadow-gray-300 hover:cursor-pointer hover:border-gray-400 hover:bg-gray-100"
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
                    <span class="font-medium">{{ user.name }}</span>
                    <span class="text-gray-500">{{ user.slug }}</span>
                  </div>
                </div>
              </template>
              <!-- Client Info -->
              <template #footer>
                <div class="px-2.5 pb-1.5 pt-2 text-gray-500">
                  <div class="flex w-full flex-row">
                    <span>{{ clientMeta.operatingSystem }}</span>
                    <span class="ml-auto">{{ clientMeta.browserName }} {{ clientMeta.browserVersion }}</span>
                  </div>
                  <div class="flex w-full flex-row text-xs">
                    <span>id:{{ client?.id.split("-")[0] }}</span>
                    <span class="ml-auto">nonce:{{ clientMeta.nonce.split("-")[0] }}</span>
                  </div>
                </div>
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
          @click="() => runAction('user.login')"
        />
      </template>
    </div>
  </div>
</template>
