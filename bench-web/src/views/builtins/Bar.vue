<script lang="ts" setup>
import { Anchor, ClientType, NodeType, Orientation } from "@/proto/wire";
import { DECLARED_ACTIONS_BY_ID, fireActionById, type Action, type ActionBuiltinId } from "@/ui/action";
import { CLIENT_TYPE, isDeveloperMode } from "@/system/client";
import { DEFAULT_USER_ICON, ICON_BY_NODE_TYPE, IconInline, makeIcon } from "@/ui/icon";
import { toCamelName } from "@/language/const";
import { bench, hasLocalBench } from "@/system/space";
import { isAuthenticated, user } from "@/system/user";
import type { FloatingPlacement } from "@/utils/floating";
import { COMMIT, ENV, VERSION } from "@/utils/globals";
import { menuActionsLike, menuItemFromAction, type MenuItem } from "@/ui/popover";
import { tooltipFromAction } from "@/ui/tooltip";
import Menu from "@/views/builtins/Menu.vue";
import Popover from "@/views/builtins/Popover.vue";
import { useElementSize } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";
import ConnectionStatus from "./ConnectionStatus.vue";

const props = defineProps<{
  anchor: Anchor;
  orientation: Orientation;
}>();

const barRef = ref<HTMLElement | null>(null);
const barSize = useElementSize(barRef, undefined, { box: "border-box" });
const dockRef = ref<HTMLElement | null>(null);
const dockSize = useElementSize(dockRef, undefined, { box: "border-box" });
const floatingPlacement: Ref<FloatingPlacement> = computed(() => {
  if (props.anchor == Anchor.LEFT) return "right";
  else if (props.anchor == Anchor.TOP) return "bottom";
  else if (props.anchor == Anchor.RIGHT) return "left";
  else if (props.anchor == Anchor.BOTTOM) return "top";
  else return "bottom";
});

const BENCH_MENU_ITEMS = computed(() => {
  const items: MenuItem[] = [
    // bench
    // NOTE :Incomplete: select bench/branch/package etc.
    // main
    {
      id: "omnibar",
      type: "generic",
      category: "main",
      icon: "fas fa-magnifying-glass",
      title: "Search",
      action: { items: menuActionsLike(["space.omnibar*", "common.search*"], { context: undefined }) },
    },
    {
      id: "view",
      type: "generic",
      category: "main",
      icon: ICON_BY_NODE_TYPE[NodeType.VIEW],
      title: "View",
      action: {
        items: menuActionsLike(["view.navigate.close*", "view.layout.*", "view.space.*"], { context: undefined }),
      },
    },
    {
      id: "edit",
      type: "generic",
      category: "main",
      icon: "fas fa-hammer",
      title: "Edit",
      action: {
        items: menuActionsLike(["common.history*", "common.edit*", "common.move*"], { context: undefined }),
      },
    },
    {
      id: "sense",
      type: "generic",
      category: "main",
      icon: "fas fa-telescope",
      title: "Analyze",
      action: {
        items: [
          menuItemFromAction("space.launch.inspect"),
          ...menuActionsLike(["common.sense*"], { context: undefined }),
        ],
      },
    },
    {
      id: "session",
      type: "generic",
      category: "main",
      icon: "fas fa-play",
      title: "Run",
      action: { items: menuActionsLike(["session.run*"], { context: undefined }) },
    },
    // extra
    menuItemFromAction("space.launch.explorer"),
    menuItemFromAction("space.launch.outline"),
    menuItemFromAction("space.launch.inspect"),
    menuItemFromAction("space.launch.logs"),
    menuItemFromAction("space.launch.create"),
    menuItemFromAction("space.launch.docs"),
    menuItemFromAction("space.launch.discord"),
  ];

  if (isDeveloperMode.value) {
    items.push({
      id: "developer",
      type: "generic",
      category: "developer",
      icon: "fas fa-binary",
      title: "Developer",
      action: { items: menuActionsLike(["developer*"], { context: undefined }) },
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

const dockActions: Ref<Action[]> = computed(
  () =>
    (
      [
        "space.omnibar.actions",
        "space.omnibar.space",
        "space.edit.inspect",
        "space.launch.create",
        "space.launch.chat",
        "space.launch.docs",
      ] as ActionBuiltinId[]
    )
      .map((id) => DECLARED_ACTIONS_BY_ID.value[id])
      .filter((a) => a != null) as Action[],
);
</script>
<template>
  <div
    ref="barRef"
    class="flex w-full gap-1.5 text-sm text-gray-900"
    :class="orientation == Orientation.HORIZONTAL ? 'flex-row items-center px-0.5' : 'flex-col items-center py-0.5'"
    data-outside-view="true"
  >
    <!-- Bench -->
    <Popover :placement="floatingPlacement" :reference-margin="4" :container-margin="4">
      <template #trigger="{ toggle, isOpen }">
        <button
          class="flex h-[30px] w-full select-none flex-row items-center rounded px-2.5 py-1 text-gray-700 hover:bg-gray-100 hover:text-primary-900"
          :class="[isOpen ? ' bg-gray-100' : '']"
          @click="toggle"
        >
          <img src="@/assets/icon_outline.svg" class="h-5 w-5 rounded-md" />
        </button>
      </template>

      <template #content="{ close }">
        <!-- Bench Menu -->
        <Menu v-outside.mousedown.stop="close" :items="BENCH_MENU_ITEMS">
          <!-- Bench Info -->
          <template v-if="bench" #header>
            <div class="flex flex-row px-2.5 pb-2 pt-1.5">
              <div class="mr-2 w-10 rounded border border-gray-300 bg-gray-100 py-0.5 text-center text-lg">
                <IconInline v-if="bench.icon" class="text-gray-700" v-bind="bench.icon" />
                <img v-else src="@/assets/icon_outline.svg" class="mx-auto h-7 w-7 rounded-md" />
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
              <span class="select-all">{{ toCamelName(ClientType, CLIENT_TYPE) }}</span>
              <span class="ml-auto select-all">{{ VERSION }}</span>
            </div>
            <div class="flex w-full flex-row px-2.5 pb-1.5 text-xs text-gray-500">
              <span class="select-all">{{ ENV ?? "dev" }}</span>
              <span class="ml-auto select-all">#{{ COMMIT?.slice(0, 8) ?? "???" }}</span>
            </div>
          </template>
        </Menu>
      </template>
    </Popover>

    <!-- Dock -->
    <div
      ref="dockRef"
      class="flex flex-shrink-0 gap-x-1 gap-y-1.5"
      :class="[orientation == Orientation.HORIZONTAL ? 'absolute flex-row' : 'flex-col']"
      :style="{
        left:
          orientation == Orientation.HORIZONTAL ? barSize.width.value / 2 - dockSize.width.value / 2 + 'px' : 'auto',
      }"
    >
      <button
        v-for="action in dockActions"
        :key="action.id"
        v-tooltip="tooltipFromAction(action, { placement: 'top', showDelay: 800, group: 'bar.dock' })"
        class="rounded px-2.5 py-1 text-gray-700 hover:bg-gray-100 hover:text-primary-900"
        @click="fireActionById(action.id)"
      >
        <IconInline class="text-base" v-bind="action.icon" />
      </button>
    </div>

    <!-- End -->
    <div
      class="flex flex-shrink-0 items-center gap-1.5"
      :class="[orientation == Orientation.HORIZONTAL ? 'ml-auto flex-row pr-1' : 'mt-auto flex-col pb-1']"
    >
      <!-- Connection -->
      <ConnectionStatus />
      <!-- Notifications -->
      <!-- Main Clients/Places (browser plugin, mobile, etc.) -->
      <!-- User Menu -->
      <template v-if="user">
        <!-- User (logged in) -->
        <Popover :placement="floatingPlacement" :reference-margin="4" :container-margin="4">
          <template #trigger="{ toggle, isOpen }">
            <button
              class="px-2 py-1 text-base text-gray-700 hover:bg-gray-100 hover:text-primary-900"
              :class="[isOpen ? 'bg-gray-100' : '']"
              @click="toggle"
            >
              <IconInline class="" v-bind="user.icon ?? makeIcon('fa fa-user-circle')" />
            </button>
          </template>
          <template #content="{ close }">
            <Menu v-outside.mousedown.stop="close" :items="USER_MENU_ITEMS">
              <!-- User Info -->
              <template #header>
                <div class="flex flex-row px-2.5 pb-2 pt-1.5">
                  <div class="mr-2 rounded border border-gray-200 bg-gray-100 px-2.5 py-0.5 text-xl">
                    <IconInline class="text-gray-700" v-bind="user.icon ?? DEFAULT_USER_ICON" />
                  </div>
                  <div class="flex flex-col leading-tight">
                    <span class="select-all font-medium">{{ user.name }}</span>
                    <span class="select-all text-gray-500">{{ user.slug }}</span>
                  </div>
                </div>
              </template>
              <!-- Clients -->
              <!-- .. -->
            </Menu>
          </template>
        </Popover>
      </template>
      <template v-else>
        <!-- Not logged in -->
        <button
          class="px-2 py-1 text-base text-gray-700 hover:bg-gray-100 hover:text-primary-900"
          @click="fireActionById('user.auth.login')"
        >
          <i class="fas fa-arrow-right-to-bracket" />
        </button>
      </template>
    </div>
  </div>
</template>
