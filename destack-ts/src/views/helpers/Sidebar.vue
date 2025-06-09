<script lang="ts" setup>
import { NodeType, NodeTypeOptionInfo, Orientation, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { destack, canvas, hasLocalDestack } from "@/system/space";
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
import PackageTree from "@/views/helpers/NodeTree.vue";
import ThreadList from "@/views/helpers/ThreadList.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, Ref, ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const FOOTER_HEIGHT = 42;

const USER_MENU_ITEMS = computed(() => {
  const items = [menuItemFromCommand("user.navigate.goToHome", { category: "primary" })];
  if (isAuthenticated.value && !hasLocalDestack.value) {
    items.push(menuItemFromCommand("user.navigate.activate", { category: "primary" }));
  }
  items.push(...[menuItemFromCommand("user.auth.logout", { category: "secondary" })]);
  return items;
});

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size"
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const bodyRef = ref<HTMLElement | null>(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT - FOOTER_HEIGHT);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExpose>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Destack Header -->
    <div
      class="flex max-w-full shrink-0 flex-col gap-x-2"
      :style="{
        minHeight: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Destack button -->
      <div
        class="my-1.5 mr-2.5 ml-2 flex cursor-pointer flex-row items-center truncate rounded-sm px-2 transition-colors duration-75 hover:bg-gray-100"
        :data-node-id="destack?.id"
        :data-node-type="destack?.metatype"
        :style="{
          height: `${BAR_HEADER_HEIGHT - 12}px`,
        }"
        :disabled="destack == null"
        @click="destack != null && canvas.inspect({ node: destack! })"
      >
        <img class="mr-1 h-6 w-6" src="/favicon.ico" />
        <span v-if="destack" class="truncate font-medium">{{ destack.name }}</span>
        <span v-else class="italic"> Destack </span>
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
                'space.omnibar.destack',
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
          <PackageTree id="package" class="" :node-ptr="props.nodePtr" />
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
        @click="user == null && fireCommandById('user.auth.login')"
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
