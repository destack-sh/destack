<script lang="ts" setup>
import { createBlock } from "@/language/block";
import { toCamelName } from "@/language/const";
import { packSubnode, useSubnodeProperty } from "@/language/node";
import { BlockType, HubAspect, NodeType, TreeViewPreset, ViewData, ViewType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { bench, canvas, hasLocalBench, pkg, pkgConnection, pkgGraph } from "@/system/space";
import { isAuthenticated, user } from "@/system/user";
import { ICON_BY_NODE_TYPE, IconInline, makeIcon } from "@/ui/icon";
import { MenuItem, PopoverInfoIn, menuActionsLike, menuItemFromAction } from "@/ui/popover";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { isDeveloperMode } from "@/utils/globals";
import { viewEmits, type ViewExposed } from "@/views/common";
import Tree from "@/views/system/Tree.vue";
import { computed, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_BAR_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

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
        items: [...menuActionsLike(["common.sense*"], { context: undefined })],
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
    menuItemFromAction("space.launch.documentation"),
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

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size" | "subnodePacked"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const children = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const aspect = useSubnodeProperty(NodeType.VIEW, ViewType.HUB, toRef(props, "subnodePacked"), "aspect");

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      v-menu="(): PopoverInfoIn => ({ kind: 'menu', items: BENCH_MENU_ITEMS, placement: 'bottom-left' })"
      class="mx-2 my-1.5 flex cursor-pointer flex-row items-center rounded pl-2.5 pr-1 transition-colors duration-75 hover:bg-gray-100"
      :style="{
        height: `${BAR_HEADER_HEIGHT - 12}px`,
      }"
    >
      <span>
        <i class="fas fa-circle mr-2 text-primary-500" />
        <span v-if="bench" class="font-medium">
          {{ bench.name }}
        </span>
        <span v-else class="italic"> Bench </span>
      </span>
    </div>

    <!-- Header -->
    <div
      class="mx-3 my-1.5 flex flex-row items-center gap-x-2"
      :style="{
        height: `${HEADER_HEIGHT - 12}px`,
      }"
    >
      <button
        v-for="a in [HubAspect.SOURCE, HubAspect.ACTIVITY, HubAspect.EXTERNAL]"
        :key="a"
        class="flex flex-shrink-0 cursor-pointer flex-row items-center rounded px-2 py-1 transition-colors duration-75"
        :class="[
          a == aspect ? 'bg-gray-100 font-medium text-gray-900' : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700',
        ]"
        @click="state.update({ metatype: NodeType.VIEW, type: ViewType.HUB, subnode: { aspect: a } })"
      >
        <span>{{ toCamelName(HubAspect, a) }} </span>
      </button>
    </div>

    <!-- Base controls -->
    <!-- ... -->

    <div v-if="aspect == HubAspect.SOURCE">
      <!-- Main tree -->
      <div
        class="mx-4 mt-1.5 flex flex-row items-center"
        :style="{
          height: `${HEADER_HEIGHT - 6}px`,
        }"
      >
        <span class="font-medium">Pages</span>
        <!-- Create -->
        <button
          v-if="pkg != null"
          class="ml-auto rounded px-1.5 py-0.5 text-gray-400 transition-colors duration-75 hover:bg-gray-200 hover:text-gray-700"
          @click.stop="
            () => {
              if (pkg == null) return;
              const block = createBlock(pkgConnection.tx, pkgGraph, {
                anchor: 'inside',
                target: pkg,
                block: { type: BlockType.PAGE },
              });
              canvas.goToNode(block);
            }
          "
        >
          <i class="fas fa-plus" />
        </button>
      </div>
      <Tree
        id="explore"
        class=""
        :node-ptr="props.nodePtr"
        :subnode-packed="packSubnode(NodeType.VIEW, ViewType.TREE, { preset: TreeViewPreset.EXPLORE })"
        size-is-dynamic
        v-bind="state.getChildState('explore')"
      />
      <!-- Outline -->
      <div
        class="mx-4 mt-1.5 flex flex-row items-center"
        :style="{
          height: `${HEADER_HEIGHT - 6}px`,
        }"
      >
        <span class="font-medium">Outline</span>
      </div>
      <Tree
        id="outline"
        class=""
        :node-ptr="props.nodePtr"
        :subnode-packed="packSubnode(NodeType.VIEW, ViewType.TREE, { preset: TreeViewPreset.OUTLINE })"
        size-is-dynamic
        v-bind="state.getChildState('outline')"
      />
    </div>

    <!-- User -->
    <div class="absolute bottom-0 z-10 flex h-[44px] w-full flex-row items-center border-gray-200">
      <button
        v-menu="(): PopoverInfoIn => ({ kind: 'menu', items: USER_MENU_ITEMS, placement: 'top-left' })"
        class="mx-2 w-full py-1 pl-2.5 pr-1.5 text-left text-gray-700 transition-colors duration-75 hover:bg-gray-100"
      >
        <IconInline class="" v-bind="user.icon ?? makeIcon('fa fa-user-circle')" />
        <span v-if="user" class="ml-2">{{ user.name }}</span>
        <span v-else class="ml-2">Log In</span>
      </button>
    </div>
  </div>
</template>
