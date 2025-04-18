<script lang="ts" setup>
import { packSubnode, useSubnodeProperty } from "@/language/core/node";
import { NodeType, NodeTypeOptionInfo, Orientation, ViewData, ViewType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { bench, benchConnection, canvas, hasLocalBench, spaceGraph } from "@/system/space";
import { isAuthenticated, user, userConnection } from "@/system/user";
import { CommandBuiltinId, fireCommand, fireCommandById, getCommand } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { AvatarInline, getNodeIcon, IconInline, makeIcon } from "@/ui/icon";
import { menuItemFromCommand, PopoverInfoIn } from "@/ui/popover";
import { Shortcut } from "@/ui/tooltip";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import PackageTree from "@/views/helpers/PackageTree.vue";
import ThreadList from "@/views/helpers/ThreadList.vue";
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
const expandedPackageNodesPtr = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.SIDEBAR,
  toRef(props, "subnodePacked"),
  "expandedPackageNodesPtr",
);

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
      class="flex max-w-full shrink-0 flex-col gap-x-2"
      :style="{
        minHeight: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Bench button -->
      <div
        class="mx-2.5 my-1.5 flex cursor-pointer flex-row items-center truncate rounded-sm px-2 transition-colors duration-75 hover:bg-gray-100"
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
        <span v-if="bench" class="truncate font-medium">{{ bench.name }}</span>
        <span v-else class="italic"> Bench </span>
      </div>
    </div>

    <!-- Body -->
    <Scroll
      id="scroll"
      ref="scrollRef"
      :orientation="Orientation.VERTICAL"
      :size="{ width: size?.width, height: bodyHeight }"
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZone, e)"
    >
      <div
        ref="bodyRef"
        class="flex flex-col gap-y-2"
        :style="{
          maxWidth: size?.width != null ? `${size.width}px` : undefined,
          minHeight: `${bodyHeight - 10 /* not entirely sure why, the Scroll component seems to have some padding/border? */}px`,
        }"
      >
        <!-- Quick/Global commands -->
        <div class="flex flex-col">
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
            class="group/button mx-3 flex cursor-pointer flex-row items-center truncate rounded-sm border border-transparent px-1.5 py-[4px] transition-colors duration-75 hover:bg-gray-100"
            :style="{}"
            @click="fireCommand(command)"
          >
            <IconInline class="mr-1 w-5 text-center" v-bind="command.icon" />
            <span class="">
              {{ command.title }}
            </span>
            <span class="ml-auto">
              <!-- Shortcut -->
              <Shortcut
                v-if="command.shortcuts?.length ?? 0 > 0"
                class="text-gray-400 opacity-0 transition-colors duration-75 group-hover/button:opacity-100"
                :shortcut="command.shortcuts![0]"
              />
            </span>
          </button>
        </div>

        <!-- Package -->
        <div class="group/package-tree">
          <div class="group/header mx-3 my-1 flex flex-row items-center px-1.5 py-0.5">
            <span class="font-medium">Pages</span>
            <!-- Commands -->
            <button
              v-tooltip="{
                small: true,
                text: 'Create Page',
                shortcuts: getCommand('space.create.page').shortcuts,
              }"
              class="ml-auto cursor-pointer rounded-sm text-gray-400 opacity-0 transition-colors duration-75 group-hover/package-tree:opacity-100 hover:bg-gray-100 hover:text-gray-700"
              @click="fireCommandById('space.create.page')"
            >
              <span class="fas fa-plus w-5 text-center" />
            </button>
          </div>
          <PackageTree
            id="package"
            class=""
            :node-ptr="props.nodePtr"
            :expanded-nodes-ptr="expandedPackageNodesPtr"
            @update:expanded-nodes-ptr="
              state.update(
                {
                  subnodePacked: packSubnode(NodeType.VIEW, ViewType.SIDEBAR, {
                    expandedPackageNodesPtr: $event,
                  }),
                },
                { debounce: 'long' },
              )
            "
          />
        </div>

        <!-- Threads -->
        <div>
          <ThreadList id="threads" />
        </div>
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
        class="mx-3 flex cursor-pointer flex-row items-center rounded-sm py-1 pr-1.5 pl-1.5 text-left transition-colors duration-75 hover:bg-gray-100"
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
