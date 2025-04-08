<script lang="ts" setup>
import { makeType, makeTypeConstraint } from "@/language/core/type";
import {
  BenchType,
  ColorData,
  ColorShade,
  ColorType,
  FileType,
  IconData,
  IconType,
  NodeReferenceData,
  NodeType,
  Orientation,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { getIconMetadata, IconInline, makeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import type { PopoverInfoIn } from "@/ui/popover";
import { EMOJI_ICON_INDEX, FONT_AWESOME_ICON_INDEX, SearchIndex, useIndexSearch, type IconItem } from "@/ui/search";
import { getColorHex } from "@/ui/style";
import type { TooltipInfo } from "@/ui/tooltip";
import { type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const DEFAULT_WIDTH = 380;
const MAX_HEIGHT = 280;
const ITEMS_PER_ROW = 10;
const MAX_ROWS = 32;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; modelValue?: IconData } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "isInput" | "isInline" | "isDisabled" | "valueType">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

// state
const iconType: Ref<IconType> = ref(IconType.EMOJI);
const query: Ref<string> = ref("");
const activeResultId: Ref<string | null> = ref(null);
const color: Ref<ColorData | null> = ref(props.modelValue?.color ?? null);
const effectiveColorType = computed(() => color.value?.type ?? ColorType.GRAY);
const effectiveColorHex = computed(() => {
  if (effectiveColorType.value == ColorType.GRAY) return getColorHex(ColorType.GRAY, ColorShade.S700);
  else return getColorHex(effectiveColorType.value, ColorShade.S400);
});

// view
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const headerRef: Ref<HTMLDivElement | null> = ref(null);

const indices: Ref<Record<string, SearchIndex<IconItem>>> = computed(() => {
  const indices: Record<string, SearchIndex<IconItem>> = {};
  if (iconType.value == IconType.FONT_AWESOME) {
    indices["icon-fa"] = FONT_AWESOME_ICON_INDEX;
  } else if (iconType.value == IconType.EMOJI) {
    indices["icon-emoji"] = EMOJI_ICON_INDEX;
  }
  return indices;
});
const { results, resultsTotal } = useIndexSearch<IconItem>({
  query,
  indices,
  isEnabled: computed(() => props.isInline),
  options: { outOfOrder: 0, highlight: false, maxResults: MAX_ROWS * ITEMS_PER_ROW },
});

// auto-select best match when searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultId.value = results.value[0].id;
  }
});

function isSelected(item: IconItem) {
  if (item.icon == null) {
    return false;
  } else if (item.icon.faName != null) {
    return item.icon.faName == props.modelValue?.faName;
  } else if (item.icon.emoji != null) {
    return item.icon.emoji == props.modelValue?.emoji;
  } else {
    return false;
  }
}

function fire(item: IconItem) {
  const icon = { ...item.icon, color: color.value ?? undefined };
  apply(icon);
}
function apply(icon: IconData | undefined) {
  emit("update:modelValue", icon);
  emit("apply", icon);
}

function focus(anchor?: "up" | "down" | "left" | "right" | FocusAnchor | NodeReferenceData) {
  if (props.isInline) {
    let nextIdx;
    const currentIdx = results.value.findIndex((item) => item.id === activeResultId.value);
    if (anchor === "up") {
      nextIdx = currentIdx - ITEMS_PER_ROW;
    } else if (anchor === "down") {
      nextIdx = currentIdx + ITEMS_PER_ROW;
    } else if (anchor === "left") {
      nextIdx = currentIdx - 1;
    } else if (anchor === "right") {
      nextIdx = currentIdx + 1;
    } else {
      nextIdx = 0;
    }

    nextIdx = Math.max(0, Math.min(results.value.length - 1, nextIdx));
    activeResultId.value = results.value[nextIdx].id;
    if (resultsRefs.value[activeResultId.value] != null) {
      resultsRefs.value[activeResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
    }
    queryRef.value?.focus();
  } else {
    return false;
  }
}

canvas.registerView(self, id);
defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <button
    v-if="!isInline"
    v-menu="
      (): PopoverInfoIn => ({
        kind: 'view',
        component: ViewType.ICON,
        placement: 'bottom-left',
        offset: 'referenceWidth',
        props: { ...props, isInline: true },
        onApply: (value) => apply(value),
      })
    "
    :disabled="isDisabled || !isInput"
    class="group flex w-full flex-row items-center rounded border border-gray-200 px-2 py-1 enabled:hover:border-gray-200 data-[popover=true]:border-gray-200 data-[popover=true]:text-primary-700"
  >
    <!-- TODO :Incomplete: Icon.isInput/isDisabled/variants/... -->
    <!-- Dropdown -->
    <template v-if="modelValue != null">
      <IconInline v-bind="modelValue" />
      <span class="ml-1.5">{{ getIconMetadata(modelValue)?.title ?? "Custom Icon" }}</span>
    </template>
    <template v-else>
      <i class="fas fa-icons text-gray-400 group-hover:text-gray-700" />
      <span class="ml-1.5 text-gray-400 group-hover:text-gray-700">Select Icon</span>
    </template>
    <i v-if="!isDisabled && isInput" class="fas fa-caret-down ml-auto pl-1.5 text-gray-400" />
  </button>

  <div v-else-if="isInline" :style="{ width: DEFAULT_WIDTH + 'px' }">
    <!-- Inline Combobox -->
    <!-- Header -->
    <div ref="headerRef" class="flex w-full flex-col gap-y-0.5 border-b border-gray-200 px-2 py-2">
      <!-- Select -->
      <div class="flex flex-row gap-x-1.5 py-1">
        <!-- Change Icon type -->
        <button
          v-for="{ type, title } in [
            {
              type: IconType.EMOJI,
              title: 'Emoji',
            },
            {
              type: IconType.FONT_AWESOME,
              title: 'Icon',
            },
            {
              type: IconType.FILE,
              title: 'File',
            },
          ]"
          :key="type"
          class="rounded px-1.5 py-0.5 font-medium transition-colors duration-150 enabled:hover:bg-gray-100"
          :class="[type == iconType ? 'bg-gray-100 text-gray-700' : 'text-gray-400 hover:text-gray-700']"
          @click.stop.prevent="((iconType = type), queryRef?.focus())"
        >
          <span>{{ title }}</span>
        </button>
        <!-- Remove -->
        <button
          v-if="modelValue != null"
          class="ml-auto rounded px-1.5 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
          @click="apply(undefined)"
        >
          <span>Remove</span>
        </button>
      </div>
      <!-- Input -->
      <div class="flex flex-row items-center rounded bg-gray-100 px-1.5 py-1">
        <!-- Query -->
        <input
          ref="queryRef"
          v-model="query"
          type="text"
          class="w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0 disabled:cursor-default"
          :disabled="iconType == IconType.FILE"
          :placeholder="
            iconType == IconType.FILE ? undefined : `Search ${iconType == IconType.EMOJI ? 'Emojis' : 'Icons'}...`
          "
          @keydown.enter.stop.prevent="activeResultId != null && fire(results.find((r) => r.id === activeResultId)!)"
          @keydown.up.stop.prevent="focus('up')"
          @keydown.down.stop.prevent="focus('down')"
          @keydown.left.stop.prevent="focus('left')"
          @keydown.right.stop.prevent="focus('right')"
        />
        <!-- Color -->
        <button
          v-if="iconType != IconType.EMOJI && iconType != IconType.FILE"
          v-tooltip="{ title: 'Change color', small: true }"
          v-menu="
            (): PopoverInfoIn => ({
              kind: 'view',
              component: ViewType.COLOR,
              placement: 'top',
              reference: headerRef!,
              props: { modelValue: color, isInput: true },
              // NOTE :UX: not sure whether changing Color in Icon picker should instantly apply to current icon
              onApply: (value) => (color = value),
            })
          "
          :disabled="isDisabled || !isInput"
          class="rounded px-0.5 enabled:hover:bg-gray-100"
        >
          <i class="fas fa-circle small" :style="{ color: effectiveColorHex }" />
        </button>
      </div>
    </div>
    <!-- File -->
    <div v-if="iconType == IconType.FILE" :style="{ width: DEFAULT_WIDTH + 'px', height: MAX_HEIGHT + 'px' }">
      <div class="p-2">
        <File
          id="file"
          class="w-full"
          :style="{ height: MAX_HEIGHT - 2 * 8 + 'px' }"
          :size="{ width: DEFAULT_WIDTH, height: MAX_HEIGHT - 2 * 8 }"
          is-inline
          is-input
          is-icon
          :value-type="
            makeType({
              kind: TypeKind.NODE,
              benchType: BenchType.FILE,
              constraint: makeTypeConstraint({ nodeSubtypes: [FileType.IMAGE] }),
            })
          "
          :model-value="modelValue?.filePtr"
          @update:model-value="
            (filePtr) => {
              if (filePtr != null) {
                apply(makeIcon({ filePtr, color: color ?? undefined }));
              } else {
                apply(undefined);
              }
            }
          "
        />
      </div>
    </div>
    <!-- Search Results -->
    <Scroll
      v-else
      id="body"
      size-is-dynamic
      :size="{ width: DEFAULT_WIDTH, height: MAX_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
    >
      <ul
        v-if="results.length > 0"
        class="grid grid-cols-10 gap-x-1 gap-y-1 px-2 py-2 text-center"
        :style="{ color: effectiveColorHex }"
      >
        <template v-for="(item, i) in results" :key="i">
          <span
            :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
            v-tooltip="
              {
                group: 'icon',
                isEnabled: results[i] != null,
                title: () => results[i]?.title /* results[i] because 'item' is captured once and never updated */,
                showDelay: 200,
                hideDelay: 100,
                small: true,
              } as TooltipInfo
            "
            class="select-none rounded border border-transparent hover:cursor-pointer hover:border-gray-200 hover:bg-gray-100 data-[active=true]:border-gray-200 data-[active=true]:bg-gray-100"
            :class="item.icon?.faName != null ? 'py-1 text-sm ' + item.icon.faName : 'text-xl'"
            role="menuitem"
            :data-selected="isSelected(item)"
            :data-active="item.id === activeResultId"
            @click.stop.prevent="fire(item)"
            @keydown.enter.stop.prevent="fire(item)"
            v-html="item.icon?.emoji"
          />
        </template>
      </ul>
      <!-- NOTE: Picker no results/overflow is very similar to Omnibar/Picker/etc :ResultInfo -->
      <!-- Too many results (truncated) -->
      <div v-if="results.length < resultsTotal" class="my-1 max-w-full px-3 pb-2 text-gray-500">
        <i class="fas fas fa-ellipsis" />
        <span class="ml-2">
          <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results
          <template v-if="query.length > 0">for </template>
          <span class="truncate font-semibold">{{ query }}</span>
        </span>
      </div>
      <!-- Help -->
      <div v-if="results.length == 0" class="max-w-full px-1 py-1">
        <!-- Nothing found -->
        <div v-if="results.length === 0" class="px-2 py-1 text-gray-500">
          <i class="fas fa-empty-set text-gray-600" />
          <span class="ml-1">
            No results
            <span v-if="query">
              for <span class="truncate font-semibold">{{ query }}</span>
            </span>
          </span>
        </div>
      </div>
    </Scroll>
  </div>
</template>
