<script lang="ts" setup>
import { makeExpression } from "@/language/core/expression";
import { packSubnode } from "@/language/core/node";
import {
  ExpressionType,
  NodeType,
  NodeTypeOptionInfo,
  Orientation,
  ThreadProperty,
  ThreadStatus,
  TreeViewPreset,
  ViewData,
  ViewType
} from "@/proto/wire";
import { propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { bench, benchConnection, canvas, hasLocalBench, spaceGraph } from "@/system/space";
import { isAuthenticated, user, userConnection } from "@/system/user";
import { CommandBuiltinId, fireCommand, fireCommandById, getCommand } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { AvatarInline, getNodeIcon, IconInline, makeIcon } from "@/ui/icon";
import { menuItemFromCommand, PopoverInfoIn } from "@/ui/popover";
import { Shortcut } from "@/ui/tooltip";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import List from "@/views/collections/List.vue";
import Tree from "@/views/collections/Tree.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, Ref, ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const FOOTER_HEIGHT = 42;

const USER_MENU_ITEMS = computed(() => {
  const items = [menuItemFromCommand("user.navigate.goToHome", { category: "primary" })];
  if (isAuthenticated.value && !hasLocalBench.value) {
    items.push(menuItemFromCommand("user.navigate.activate", { category: "primary" }));
  }
  items.push(...[menuItemFromCommand("user.security.logout", { category: "secondary" })]);
  return items;
});

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size" | "subnodePacked"
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const children = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });

const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const bodyRef = ref<HTMLElement | null>(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT - FOOTER_HEIGHT);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExpose>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 flex max-w-full flex-shrink-0 flex-col gap-x-2 pb-2"
      :style="{
        minHeight: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Bench button -->
      <div
        class="my-1.5 flex cursor-pointer flex-row items-center truncate rounded px-2 transition-colors duration-75 hover:bg-gray-100"
        :data-node-id="bench?.id"
        :data-node-type="bench?.metatype"
        :style="{
          height: `${BAR_HEADER_HEIGHT - 12}px`,
        }"
        :disabled="bench == null"
        @click="bench != null && canvas.inspect({ node: bench! })"
      >
        <IconInline
          v-tooltip="{ title: 'Change icon', small: true }"
          v-menu="
            (): PopoverInfoIn => ({
              kind: 'view',
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: (bench as any)!.icon, isInput: true },
              onApply: (newIcon) => benchConnection.tx.update(bench!, { icon: newIcon }),
            })
          "
          v-bind="bench != null ? getNodeIcon(bench) : makeIcon(NodeTypeOptionInfo[NodeType.BENCH]!.icon!)"
          role="button"
          class="mr-1 w-5 text-center"
        />
        <span v-if="bench" class="truncate font-medium">{{ bench.slug }}</span>
        <span v-else class="italic"> Bench </span>
      </div>
      <!-- Quick/Global commands -->
      <button
        v-for="command in (
          [
            'space.omnibar.bench',
            'space.omnibar.commands',
            'space.create.page',
            'space.create.thread',
          ] as CommandBuiltinId[]
        ).map(getCommand)"
        :key="command.id"
        class="group/button flex flex-row items-center truncate rounded border border-transparent px-2 py-[4px] transition-colors duration-150 hover:bg-gray-100"
        :style="{}"
        @click="fireCommand(command)"
      >
        <IconInline class="mr-1 w-5 text-center" v-bind="command.icon" />
        <span class="">
          {{ command.title }}
        </span>
        <span class="ml-auto">
          <Shortcut
            v-if="command.shortcuts?.length ?? 0 > 0"
            class="text-gray-400 transition-colors duration-150 group-hover/button:text-gray-900"
            :shortcut="command.shortcuts![0]"
          />
        </span>
      </button>
    </div>

    <!-- Content -->
    <Scroll
      id="scroll"
      ref="scrollRef"
      :orientation="Orientation.VERTICAL"
      :size="{ width: size?.width, height: bodyHeight }"
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZone, e)"
    >
      <div
        ref="bodyRef"
        class="flex flex-col"
        :style="{
          minHeight: `${bodyHeight - 10 /* not entirely sure why, the Scroll component seems to have some padding/border? */}px`,
        }"
      >
        <!-- Package -->
        <div
          class="group/header mx-4 flex flex-row items-center"
          :style="{
            height: `${HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-medium">Pages</span>
        </div>
        <Tree
          id="package"
          class=""
          :node-ptr="props.nodePtr"
          :subnode-packed="packSubnode(NodeType.VIEW, ViewType.TREE, { preset: TreeViewPreset.PACKAGE })"
          size-is-dynamic
          v-bind="state.getChildState('scroll.package')"
        />
        <!-- Threads -->
        <!-- ... -->
        <div
          class="group/header mx-4 flex flex-row items-center"
          :style="{
            height: `${HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-medium">Threads</span>
        </div>
        <List
          id="threads"
          :subnode-packed="
            packSubnode(NodeType.VIEW, ViewType.LIST, {
              queryNodeType: NodeType.THREAD,
              filter: makeExpression({
                type: ExpressionType.IN,
                propertyPtr: propertyReference(NodeType.THREAD, ThreadProperty.status),
                value: [ThreadStatus.OPEN],
              }),
            })
          "
        />
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </Scroll>

    <!-- Footer -->
    <div
      class="absolute bottom-0 z-10 flex w-full flex-col gap-y-1 border-t bg-white pt-1"
      :class="[scrollRef?.isVerticalOverflown ? 'border-gray-200' : 'border-transparent']"
      :style="{
        height: `${FOOTER_HEIGHT}px`,
      }"
    >
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
        class="mx-2 flex flex-row items-center rounded py-1 pl-2.5 pr-1.5 text-left transition-colors duration-75 hover:bg-gray-100"
        @click="user == null && fireCommandById('user.security.login')"
      >
        <AvatarInline
          v-tooltip="{ title: 'Change icon', small: true }"
          v-menu="
            (): PopoverInfoIn => ({
              kind: 'view',
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: (user as any)!.icon, isInput: true },
              onApply: (newIcon) => userConnection.tx.update(user!, { icon: newIcon }),
            })
          "
          v-bind="user != null ? getNodeIcon(user) : makeIcon(NodeTypeOptionInfo[NodeType.USER]!.icon!)"
        />
        <div class="flex flex-col">
          <!-- Username -->
          <span v-if="user" class="ml-2 font-medium">{{ user.name }}</span>
          <span v-else class="ml-2">Log In</span>
          <!-- Text -->
          <span v-if="user" class="text-gray-400">{{ user.email }}</span>
        </div>
      </button>
    </div>
  </div>
</template>
