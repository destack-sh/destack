<script lang="tsx" setup>
import { IconInline, toIconMaybe } from "@/system/icon";
import { useFloating, type FloatingPlacement } from "@/utils/floating";
import { log } from "@/utils/log";
import type { MenuInfo, MenuItem } from "@/utils/menu";
import { Shortcut } from "@/utils/tooltip";
import { useEventListener } from "@vueuse/core";
import { computed, onMounted, ref, type ComponentPublicInstance, type Ref, nextTick } from "vue";

const SHOW_NESTED_DELAY = 200;

const props = defineProps<MenuInfo & { parent?: MenuInfo; placement?: FloatingPlacement }>();
const emit = defineEmits(["close"]);

const menuRef: Ref<HTMLUListElement | null> = ref(null);
const query: Ref<string> = ref("some query");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const itemRefs: Ref<Record<number, HTMLElement | null>> = ref({});
const focusedItemIdx: Ref<number | null> = ref(null);
const activeNestedItemIdx: Ref<number | null> = ref(null);
const activeNestedItemRef: Ref<ComponentPublicInstance<any> | null> = ref(null);
const hoverItemTimeout: Ref<any | null> = ref(null);

/**
 * On hover we immediately focus the given element.
 * After SHOW_NESTED_DELAY we open the nested menu if any.
 */

function onMouseEnter(itemIdx: number) {
  focusedItemIdx.value = itemIdx;
  if (hoverItemTimeout.value != null) {
    clearTimeout(hoverItemTimeout.value);
  }
  hoverItemTimeout.value = setTimeout(() => {
    if (itemIdx == focusedItemIdx.value) {
      if (isNestedItem(props.items[itemIdx].action)) {
        openNestedMenu(itemIdx);
      } else {
        activeNestedItemIdx.value = null;
      }
    }
  }, SHOW_NESTED_DELAY);
}

function onMouseLeave(itemIdx: number) {
  if (hoverItemTimeout.value != null) {
    clearTimeout(hoverItemTimeout.value);
  }
}

/** Focus the first available non-disabled item  */
function focus(idx: number | "next" | "previous" | "top" | "bottom") {
  if (idx == "top") {
    idx = props.items.findIndex((item) => !item.isDisabled);
  } else if (idx == "bottom") {
    idx = props.items
      .slice()
      .reverse()
      .findIndex((item) => !item.isDisabled);
  } else if (idx == "next") {
    const offset = (focusedItemIdx.value ?? 0) + 1;
    idx = props.items.slice(offset).findIndex((item) => !item.isDisabled) + offset;
  } else if (idx == "previous") {
    const offset = focusedItemIdx.value ?? 1;
    const reverseIdx = props.items
      .slice(0, offset)
      .reverse()
      .findIndex((item) => !item.isDisabled);
    idx = reverseIdx == -1 ? -1 : offset - reverseIdx - 1;
  }
  if (idx != -1) {
    focusedItemIdx.value = idx;
  }
}

function isNestedItem(item: MenuItem["action"]): item is MenuInfo {
  return typeof item == "object";
}

/** Triggers the action for the given item */
function fire(itemIdx: number) {
  const item = props.items[itemIdx];
  log.debug("menu.fire", item.id);
  focusedItemIdx.value = itemIdx;
  if (typeof item.action == "object") {
    if (activeNestedItemIdx.value == itemIdx) {
      activeNestedItemIdx.value = null;
    } else {
      openNestedMenu(itemIdx);
    }
  } else {
    item.action();
    emit("close");
  }
}

function openNestedMenu(itemIdx: number) {
  activeNestedItemIdx.value = itemIdx;
  nextTick(() => {
    activeNestedItemRef.value?.focus("top");
  });
}

// position the nested menu
const { placement: nestedPlacement } = useFloating({
  floating: activeNestedItemRef,
  reference: computed(() => itemRefs.value[activeNestedItemIdx.value ?? 0]),
  enabled: computed(() => activeNestedItemIdx.value != null && activeNestedItemRef.value != null),
  options: { placement: "right-top", referenceMargin: 4 },
});

/** Navigate horizontally to open/close nested menus if relevant */
function onNavigateHorizontal(direction: "left" | "right") {
  if (focusedItemIdx.value == null) return;
  const item = props.items[focusedItemIdx.value];

  if (props.parent == null) {
    // in root menu
    if (!isNestedItem(item.action)) return;
    if (direction == "right") {
      openNestedMenu(focusedItemIdx.value);
    }
  } else {
    // in nested menu
    if (props.placement?.startsWith("left") && direction == "right") {
      emit("close");
    } else if (props.placement?.startsWith("right") && direction == "left") {
      emit("close");
    } else if (isNestedItem(item.action)) {
      openNestedMenu(focusedItemIdx.value);
    }
  }
}

// auto-focus when created?
onMounted(() => {
  queryRef.value?.focus();
});

// close when clicked outside of the menu
useEventListener("click", (e) => {
  if (!menuRef.value?.contains(e.target as Node)) emit("close");
});

defineExpose({ focus });
</script>
<template>
  <ul
    ref="menuRef"
    class="flex w-fit min-w-52 max-w-72 flex-col rounded-md border border-gray-700 bg-white py-1 text-gray-900 shadow-sm shadow-gray-700"
    role="menu"
    @keydown.escape.stop.prevent="emit('close')"
    @click.stop="queryRef?.focus()"
  >
    <!-- Magic floating query -->
    <!-- Captures focus for navigation, also enables search/highlight (not yet) -->
    <div class="relative h-0">
      <div class="absolute -top-6 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          class="w-fit min-w-0 border-0 bg-transparent font-semibold text-gray-900 decoration-2 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          v-model="query"
          @keydown.enter.stop.prevent="fire(focusedItemIdx ?? 0)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
          @keydown.right.stop.prevent="onNavigateHorizontal('right')"
          @keydown.left.stop.prevent="onNavigateHorizontal('left')"
        />
      </div>
    </div>

    <!-- Content -->
    <slot name="header" />
    <!-- Items -->
    <template v-for="(item, i) in items" :key="item.id">
      <!-- Category -->
      <div v-if="i != 0 && items[i - 1].category != item.category" class="my-1 h-[1px] w-full bg-gray-700" />
      <!-- Item -->
      <li
        :ref="(ref?: any) => ref != null ? (itemRefs[i] = ref) : (delete itemRefs[i])"
        role="menuitem"
        :data-selected="focusedItemIdx === i"
        class="mx-1 my-[1px] flex flex-row items-center rounded-md border border-transparent px-2 py-[3px]"
        :class="[
          item.isDisabled
            ? 'text-gray-500'
            : 'hover:cursor-pointer hover:bg-primary-300 data-[selected=true]:border-gray-900',
          activeNestedItemIdx == i ? 'bg-primary-200' : 'data-[selected=true]:bg-primary-300',
        ]"
        @click.prevent="(e) => !item.isDisabled && (e.stopPropagation(), fire(i))"
        @mouseenter="() => onMouseEnter(i)"
        @mouseleave="() => onMouseLeave(i)"
      >
        <!-- Icon (or placeholder) -->
        <i
          v-if="item.isLoading"
          class="fas fa-spinner-third mr-1.5 w-[18px] flex-shrink-0 animate-spin text-center no-underline"
        />
        <IconInline
          v-else-if="item.icon"
          v-bind="toIconMaybe(item.icon)!"
          :class="['mr-1.5 w-5 flex-shrink-0 text-center', item.isDisabled ? 'text-gray-400' : 'text-gray-700']"
        />
        <span v-else class="mr-1.5 w-[18px] flex-shrink-0">&nbsp;</span>
        <!-- Title -->
        <span class="truncate">{{ item.title }}</span>
        <!-- Shortcut or nested menu -->
        <i v-if="isNestedItem(item.action)" class="fas fa-chevron-right ml-auto pl-4 pr-1 text-gray-700" />
        <Shortcut v-else-if="(item.shortcuts?.length ?? 0) > 0" class="ml-auto pl-4" :shortcut="item.shortcuts?.[0]!" />
      </li>
    </template>
    <slot name="footer" />

    <!-- Nested menu  -->
    <Transition
      enter-active-class="transition-all ease-in duration-75"
      enter-from-class="opacity-0 scale-95"
      enter-to-class="opacity-100 scale-100"
      leave-active-class="transition-all ease-out duration-75"
      leave-from-class="opacity-100 scale-100"
      leave-to-class="opacity-0 scale-95"
    >
      <Menu
        v-if="activeNestedItemIdx != null"
        ref="activeNestedItemRef"
        :parent="props"
        :placement="nestedPlacement ?? undefined"
        v-bind="(items[activeNestedItemIdx]!.action as MenuInfo)"
        @close="(queryRef?.focus(), focus(activeNestedItemIdx)), (activeNestedItemIdx = null)"
      />
    </Transition>
  </ul>
</template>
