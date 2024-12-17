<script lang="ts" setup>
import { createBlock } from "@/language/block";
import { toCamelName } from "@/language/const";
import { packSubnode, useSubnodeProperty } from "@/language/node";
import { BlockType, HubAspect, NodeType, Orientation, TreeViewPreset, ViewData, ViewType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { CLEAR_RUN_ACTION, getRunActions, runtime } from "@/system/runtime";
import { bench, canvas, hasLocalBench, pkg, pkgConnection, pkgGraph, spaceGraph } from "@/system/space";
import { isAuthenticated, user } from "@/system/user";
import { fireActionById } from "@/ui/action";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { ICON_BY_HUB_ASPECT, ICON_BY_NODE_TYPE, IconInline, makeIcon } from "@/ui/icon";
import { menuActionsLike, MenuItem, menuItemFromAction, PopoverInfoIn } from "@/ui/popover";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { IS_DEVELOPER_MODE } from "@/utils/globals";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunStatus from "@/views/builtins/RunStatus.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Tree from "@/views/system/Tree.vue";
import { computed, Ref, ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_BAR_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const footerHeight = computed(() => (runtime.focusedRun != null ? 70 : 38));

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
      action: { items: menuActionsLike(["space.omnibar*", "space.search*"], { context: undefined }) },
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
        items: menuActionsLike(["space.history*", "space.edit*", "space.move*"], { context: undefined }),
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

  if (IS_DEVELOPER_MODE.value) {
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
    menuItemFromAction("user.navigate.goToHome", { category: "primary" }),
    menuItemFromAction("space.launch.notifications", { category: "primary" }),
  ];
  if (isAuthenticated.value && !hasLocalBench.value) {
    items.push(menuItemFromAction("user.navigate.activate", { category: "primary" }));
  }
  items.push(
    ...[
      menuItemFromAction("user.security.logout", { category: "secondary" }),
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

const children = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const aspect = useSubnodeProperty(NodeType.VIEW, ViewType.HUB, toRef(props, "subnodePacked"), "aspect");
const visibleAspects = [HubAspect.BENCH, HubAspect.ACTIVITY, HubAspect.CATALOG];
const visibleAspectsOverflow = computed(() => visibleAspects.length * 75 > (props.size?.width ?? 0));

const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const bodyRef = ref<HTMLElement | null>(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT - HEADER_HEIGHT - footerHeight.value);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      v-menu="(): PopoverInfoIn => ({ kind: 'menu', items: BENCH_MENU_ITEMS, placement: 'bottom-left' })"
      class="mx-2 my-1.5 flex flex-shrink-0 cursor-pointer flex-row items-center rounded pl-2.5 pr-1 transition-colors duration-75 hover:bg-gray-100"
      :style="{
        height: `${BAR_HEADER_HEIGHT - 12}px`,
      }"
    >
      <span>
        <i class="fas fa-circle mr-2 text-primary-500" />
        <span v-if="bench" class="font-medium"> {{ bench.slug }}'s Bench </span>
        <span v-else class="italic"> Bench </span>
      </span>
    </div>

    <!-- Header -->
    <div
      class="mx-3 my-1.5 flex flex-shrink-0 flex-row items-center gap-x-2"
      :class="[visibleAspectsOverflow ? 'justify-between' : '']"
      :style="{
        height: `${HEADER_HEIGHT - 12}px`,
      }"
    >
      <button
        v-for="a in visibleAspects"
        :key="a"
        v-tooltip="{
          small: true,
          text: toCamelName(HubAspect, a),
          isEnabled: visibleAspectsOverflow,
          group: 'hub-aspect',
        }"
        class="flex flex-shrink-0 cursor-pointer flex-row items-center rounded px-2 py-1 transition-colors duration-75"
        :class="[
          a == aspect ? 'bg-gray-100 font-medium text-gray-900' : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700',
          visibleAspectsOverflow ? 'flex-1 justify-center' : '',
        ]"
        @click="
          state.update({ metatype: NodeType.VIEW, type: ViewType.HUB, subnode: { aspect: a } }, { debounce: 'short' })
        "
      >
        <IconInline v-if="visibleAspectsOverflow" v-bind="ICON_BY_HUB_ASPECT[a]" />
        <span v-else>{{ toCamelName(HubAspect, a) }} </span>
      </button>
    </div>

    <!-- Content: Bench -->
    <div v-if="aspect == HubAspect.BENCH">
      <Scroll
        id="scroll"
        ref="scrollRef"
        :orientation="Orientation.VERTICAL"
        :size="{ width: size?.width, height: bodyHeight }"
        @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
      >
        <div
          ref="bodyRef"
          :style="{
            minHeight: `${bodyHeight - 10 /* not entirely sure why, the Scroll component seems to have some padding/border? */}px`,
          }"
        >
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
            v-bind="state.getChildState('scroll.explore')"
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
            v-bind="state.getChildState('scroll.outline')"
          />
          <!-- Selection overlay -->
          <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
        </div>
      </Scroll>
    </div>
    <div v-else class="mx-5">
      <span class="text-red-600">{{ toCamelName(HubAspect, aspect) }}</span>
    </div>

    <!-- Footer -->
    <div
      class="absolute bottom-0 z-10 flex w-full flex-col gap-y-1 border-t bg-white pt-1"
      :class="[scrollRef?.isOverflown ? 'border-gray-200' : 'border-transparent']"
      :style="{
        height: `${footerHeight}px`,
      }"
    >
      <!-- Focused Run -->
      <div
        v-if="runtime.focusedRun != null"
        role="button"
        class="mx-2 flex cursor-pointer flex-row items-center gap-x-2.5 rounded py-1 pl-2 pr-2 hover:bg-gray-100"
        @click="runtime.focusedRun != null && canvas.goToNode(runtime.focusedRun)"
      >
        <NodeReference v-if="runtime.focusedRunTree.base" light size="regular" :node="runtime.focusedRunTree.base" />
        <span v-else class="text-gray-400">Run</span>
        <!-- Status -->
        <RunStatus :run="runtime.focusedRun" />
        <!-- Controls -->
        <div class="ml-auto flex flex-row gap-x-1">
          <button
            v-for="action in [...getRunActions(runtime.focusedRun), CLEAR_RUN_ACTION]"
            :key="action.title"
            v-tooltip="{ title: action.title, small: true, group: 'run' }"
            class="rounded px-1 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="action.action()"
          >
            <IconInline v-bind="action.icon" />
          </button>
        </div>
      </div>
      <!-- User -->
      <button
        v-menu="
          (): PopoverInfoIn => ({
            kind: 'menu',
            items: USER_MENU_ITEMS,
            isEnabled: user != null,
            placement: 'top-left',
          })
        "
        class="mx-2 rounded py-1 pl-2.5 pr-1.5 text-left transition-colors duration-75 hover:bg-gray-100"
        @click="user == null && fireActionById('user.security.login')"
      >
        <IconInline class="text-gray-700" v-bind="user?.icon ?? makeIcon('fa fa-user-circle')" />
        <span v-if="user" class="ml-2">{{ user.name }}</span>
        <span v-else class="ml-2">Log In</span>
      </button>
    </div>
  </div>
</template>
