<script lang="tsx" setup>
import { IconInline, toIconMaybe } from "@/system/icon";
import { useFloating, type FloatingPlacement } from "@/utils/floating";
import { log } from "@/utils/log";
import type { MenuInfo, MenuItem } from "@/utils/menu";
import { Shortcut } from "@/utils/tooltip";
import { useEventListener } from "@vueuse/core";
import { computed, onMounted, ref, type ComponentPublicInstance, type Ref, nextTick, shallowRef, watch } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { highlightMatches } from "@/system/search";

const SHOW_NESTED_DELAY = 200;

const props = defineProps<MenuInfo & { parent?: MenuInfo; placement?: FloatingPlacement }>();
const emit = defineEmits<{
  close: [bubble?: boolean];
}>();

const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const itemRefs: Ref<Record<number, HTMLElement | null>> = ref({});
const focusedItemIdx: Ref<number | null> = ref(null);

const activeNestedItemIdx: Ref<number | null> = ref(null);
const activeNestedItemRef: Ref<ComponentPublicInstance<any> | null> = ref(null);
const hoverItemTimeout: Ref<any | null> = ref(null);

const itemTitleMarked: Ref<(string | null)[]> = shallowRef([]);

function isNestedItem(item: MenuItem["action"]): item is MenuInfo {
  return typeof item == "object";
}

/**
 * On hover we immediately focus the given element.
 * Then we transition nested menus as needed.
 */

function onMouseEnter(itemIdx: number) {
  if (props.items[itemIdx].isDisabled) return;

  // immediately focus
  focusedItemIdx.value = itemIdx;

  if (hoverItemTimeout.value != null) clearTimeout(hoverItemTimeout.value);
  /**  */
  const openOrCloseFocused = () => {
    if (itemIdx == focusedItemIdx.value) {
      if (isNestedItem(props.items[itemIdx].action)) {
        openNestedMenu(itemIdx);
      } else {
        activeNestedItemIdx.value = null;
      }
    }
  };
  // wait to show/hide nested menu if we're transitioning between having it open vs closed
  if ((activeNestedItemIdx.value != null) !== isNestedItem(props.items[itemIdx].action)) {
    hoverItemTimeout.value = setTimeout(openOrCloseFocused, SHOW_NESTED_DELAY);
  } else {
    openOrCloseFocused();
  }
}

function onMouseLeave(itemIdx: number) {
  if (hoverItemTimeout.value != null) {
    clearTimeout(hoverItemTimeout.value);
  }
}

/** Focus the first available non-disabled item  */
function focus(idx: number | "next" | "previous" | "top" | "bottom") {
  queryRef.value?.focus();
  if (idx == "top") {
    idx = props.items.findIndex((item) => !item.isDisabled);
  } else if (idx == "bottom") {
    idx = props.items
      .slice()
      .reverse()
      .findIndex((item) => !item.isDisabled);
  } else if (idx == "next") {
    const offset = (focusedItemIdx.value ?? 0) + 1;
    const forwardIdx = props.items.slice(offset).findIndex((item) => !item.isDisabled);
    idx = forwardIdx == -1 ? -1 : offset + forwardIdx;
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

function clear() {
  query.value = "";
  focusedItemIdx.value = null;
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
    item.action(props);
    emit("close", true);
  }
}

/** Open and focus the nested menu at the given item */
function openNestedMenu(itemIdx: number) {
  activeNestedItemIdx.value = itemIdx;
  nextTick(() => {
    activeNestedItemRef.value.clear();
    activeNestedItemRef.value.focus("top");
  });
}

/** Navigate horizontally to open/close nested menus if relevant */
function onNavigateHorizontal(direction: "left" | "right") {
  const item = focusedItemIdx.value != null ? props.items[focusedItemIdx.value] : null;

  if (props.parent == null) {
    // in root menu
    if (item != null && !isNestedItem(item.action)) return;
    openNestedMenu(focusedItemIdx.value!);
  } else {
    // in nested menu
    if (item != null && isNestedItem(item.action)) {
      openNestedMenu(focusedItemIdx.value!);
    } else if (props.placement?.startsWith("left") && direction == "right") {
      emit("close");
    } else if (props.placement?.startsWith("right") && direction == "left") {
      emit("close");
    }
  }
}

// auto-focus when created
onMounted(() => {
  // NOTE: We must focus in the *next* tick even though we're already mounted.
  //  Chromium has a bug where it gets confused about the actual position of the containing elements (I think?)
  //    when this is used as part of a popover, which breaks our floating positioning.
  nextTick(() => queryRef.value?.focus());
});

// highlight and focus best match when typing
const uf = new uFuzzy({ intraMode: 1 });
watch(
  [query],
  () => {
    itemTitleMarked.value = [];
    if (!query.value) return;

    // highlight
    const { markedResults, bestMatches } = highlightMatches({
      uf,
      query: query.value,
      candidates: props.items.map((item) => item.title),
    });
    itemTitleMarked.value = markedResults;

    // auto-select best match
    const bestMatch = bestMatches.find((i) => !props.items[i].isDisabled);
    if (bestMatch != null) focus(bestMatch);
  },
  { immediate: true },
);

// position the nested menu
const { placement: nestedPlacement } = useFloating({
  floating: activeNestedItemRef,
  reference: computed(() => itemRefs.value[activeNestedItemIdx.value ?? 0]),
  enabled: computed(() => activeNestedItemIdx.value != null && activeNestedItemRef.value != null),
  options: { placement: "right-top", referenceMargin: 8, referenceOffset: { x: 0, y: -7 } },
});

defineExpose({ focus, clear, query });
</script>
<template>
  <ul
    class="flex min-w-60 max-w-[360px] flex-col rounded-md border border-gray-700 bg-white py-1 text-gray-900 shadow-md shadow-gray-700"
    role="menu"
    @keydown.escape.stop.prevent="emit('close')"
    @click.stop="queryRef?.focus()"
    v-outside.mousedown.stop="() => emit('close')"
  >
    <!-- Magic floating query -->
    <!-- Captures focus for navigation & typing for search/highlight -->
    <div class="relative">
      <div class="absolute -top-5 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          class="max-w-60 cursor-default rounded-md border-0 bg-transparent font-semibold text-gray-900 decoration-2 underline-offset-2 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          v-model="query"
          spellcheck="false"
          @keydown.enter.stop.prevent="fire(focusedItemIdx ?? 0)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
          @keydown.right.stop.prevent="onNavigateHorizontal('right')"
          @keydown.left.stop.prevent="onNavigateHorizontal('left')"
          @click.stop="/* floating input is not meant to be 'in' the menu */ emit('close')"
        />
      </div>
    </div>

    <!-- Header -->
    <div v-if="$slots.header" class="mb-1 border-b border-gray-900">
      <slot name="header" :focus="focus" />
    </div>
    <!-- Items -->
    <template v-for="(item, i) in items" :key="item.id">
      <!-- Category -->
      <div v-if="i != 0 && items[i - 1].category != item.category" class="my-1 h-[1px] w-full bg-gray-700" />
      <!-- Item -->
      <li
        :ref="(ref?: any) => ref != null ? (itemRefs[i] = ref) : (delete itemRefs[i])"
        role="menuitem"
        :data-selected="focusedItemIdx === i"
        class="mx-1 my-0.5 flex h-[28px] flex-row items-center rounded-md border border-transparent px-2"
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
        <span class="select-none truncate" v-html="itemTitleMarked[i] ?? item.title" />
        <!-- Shortcut or nested menu -->
        <i
          v-if="isNestedItem(item.action)"
          :class="['fas fa-chevron-right ml-auto pl-4 pr-1', item.isDisabled ? 'text-gray-400' : 'text-gray-700']"
        />
        <Shortcut
          v-else-if="(item.shortcuts?.length ?? 0) > 0"
          :class="['ml-auto pl-4', item.isDisabled ? 'text-gray-400' : 'text-gray-700']"
          :shortcut="item.shortcuts?.[0]!"
          :is-disabled="item.isDisabled"
        />
      </li>
    </template>
    <!-- Filler -->
    <div v-if="items.length == 0" class="px-2.5 py-1">
      <!-- Empty state -->
      <span class="text-gray-500">Nothing here</span>
    </div>
    <!-- Footer -->
    <div v-if="$slots.footer" class="mt-1 border-t border-gray-900">
      <slot name="footer" :focus="focus" />
    </div>

    <!-- Nested menu  -->
    <Transition
      enter-active-class="transition-all ease-in duration-75"
      enter-from-class="opacity-0 translate-y-[-6px]"
      enter-to-class="opacity-100 scale-100 translate-y-0"
      leave-active-class="transition-all ease-out duration-75"
      leave-from-class="opacity-100 scale-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-[6px]"
    >
      <Menu
        v-if="activeNestedItemIdx != null"
        ref="activeNestedItemRef"
        class="absolute"
        :parent="props"
        :placement="nestedPlacement ?? undefined"
        v-bind="(items[activeNestedItemIdx]!.action as MenuInfo)"
        @close="
          (bubble) => {
            queryRef?.focus();
            focus(activeNestedItemIdx!);
            activeNestedItemIdx = null;
            if (bubble) emit('close', true);
          }
        "
      />
    </Transition>
  </ul>
</template>
