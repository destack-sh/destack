<script lang="tsx" setup>
import { IconInline, toIconMaybe } from "@/system/icon";
import type { MenuInfo } from "@/utils/menu";
import { Shortcut } from "@/utils/tooltip";
import { onMounted, ref, type Ref } from "vue";

const props = defineProps<MenuInfo>();

const query: Ref<string> = ref("some query");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const focusedItemIdx: Ref<number | null> = ref(null);

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
    const offset = (focusedItemIdx.value ?? 1);
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

onMounted(() => queryRef.value?.focus());
</script>
<template>
  <ul
    class="flex w-fit min-w-52 max-w-72 flex-col rounded-md border border-gray-700 bg-white py-1 text-gray-900 shadow-sm shadow-gray-700"
    role="menu"
  >
    <slot name="header" />

    <!-- Magic floating query -->
    <!-- Captures focus for navigation & enables search/highlight -->
    <div class="relative h-0">
      <div class="absolute -top-6 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          class="w-fit min-w-0 border-0 bg-transparent font-semibold text-gray-900 caret-transparent outline-none ring-0 focus:ring-0"
          v-model="query"
          @keydown.enter.prevent
          @keydown.up.prevent="focus('previous')"
          @keydown.down.prevent="focus('next')"
        />
      </div>
    </div>

    <!-- Items -->
    <template v-for="(item, i) in items" :key="item.id">
      <!-- Category -->
      <div v-if="i != 0 && items[i - 1].category != item.category" class="my-1 h-[1px] w-full bg-gray-700" />
      <!-- Item -->
      <li
        role="menuitem"
        :data-selected="focusedItemIdx === i"
        class="mx-1 my-[1px] flex flex-row items-center rounded-md border border-transparent px-2 py-[3px] hover:cursor-pointer"
        :class="[
          item.isDisabled
            ? 'text-gray-500'
            : 'hover:cursor-pointer hover:bg-primary-300 data-[selected=true]:border-gray-900 data-[selected=true]:bg-primary-300',
        ]"
        @click="typeof item.action == 'function' && !item.isDisabled && item.action()"
      >
        <!-- Icon (or placeholder) -->
        <i
          v-if="item.isLoading"
          class="fas fa-spinner-third mr-1.5 w-[18px] flex-shrink-0 animate-spin text-center no-underline"
        />
        <IconInline
          v-else-if="item.icon"
          v-bind="toIconMaybe(item.icon)!"
          :class="['mr-1.5 w-5 flex-shrink-0 text-center', item.isDisabled ? 'text-gray-500' : 'text-gray-700']"
        />
        <span v-else class="mr-1.5 w-[18px] flex-shrink-0">&nbsp;</span>
        <!-- Title -->
        <span class="truncate">{{ item.title }}</span>
        <!-- Shortcut or nested menu -->
        <i v-if="typeof item.action == 'object'" class="fas fa-chevron-right ml-auto pl-4 pr-1 text-gray-700" />
        <Shortcut v-else-if="(item.shortcuts?.length ?? 0) > 0" class="ml-auto pl-4" :shortcut="item.shortcuts?.[0]!" />
      </li>
    </template>

    <slot name="footer" />
  </ul>
</template>
