<script lang="ts" setup>
import { isNodeType, toCamelName } from "@/language/const";
import { getConstrainedTypeName } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import {
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  PickerVariant,
  RectangleData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { autoloader } from "@/system/globals";
import { canvas } from "@/system/space";
import {
  ICON_BY_BENCH_TYPE,
  ICON_BY_BLOCK_TYPE,
  ICON_BY_NODE_TYPE,
  ICON_BY_TYPE_KIND,
  IconInline,
  makeIcon,
} from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import type { PopoverInfoIn } from "@/ui/popover";
import type { SearchItem } from "@/ui/search";
import { useValueSearch } from "@/ui/search";
import { computedValue, toValueRef } from "@/utils/ref";
import { ViewContentWrapper, viewEmits, type FocusAnchor, type ViewExposed, type ViewProps } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 400;
const MAX_HEIGHT = 360;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: any;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    placeholder?: string;
    isPopover?: boolean;
  } & Partial<
    Pick<
      ViewData,
      "name" | "title" | "icon" | "valueType" | "isInput" | "isInline" | "isDisabled" | "isMinimal" | "subnodePacked"
    >
  >
>();
const subnodePacked = toRef(props, "subnodePacked");
const variant = useSubnodeProperty(NodeType.VIEW, ViewType.PICKER, subnodePacked, "variant");
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));

//
// Type/Value
//

const baseType = supergraph.getRef(
  computed(() => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK> | undefined),
);
const facetIcon = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return ICON_BY_BLOCK_TYPE[baseType.value.type];
  } else if (props.valueType?.benchType != null) {
    return ICON_BY_BENCH_TYPE[props.valueType.benchType];
  } else if (props.valueType?.kind != null) {
    return ICON_BY_TYPE_KIND[props.valueType.kind];
  } else {
    return null;
  }
});
const facetName = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return baseType.value.name;
  } else if (props.valueType?.benchType != null) {
    return getConstrainedTypeName(props.valueType);
  } else if (props.valueType?.kind == TypeKind.NODE) {
    return toCamelName(TypeKind, props.valueType.kind);
  } else {
    return null;
  }
});

// register node references in supergraph (for autoloading / reactivity)
// NOTE :Architecture: needing to subscribe to supergraph for nodes (e.g., in Picker) seems unwieldy
//  (but we need to signal to autoloader somehow that we need these nodes loaded...)
const nodePtrs: Ref<NodeReferenceData[]> = computedValue(() => {
  if (
    props.modelValue == null ||
    props.valueType == null ||
    (props.valueType.kind != TypeKind.NODE && !isNodeType(props.valueType.benchType))
  ) {
    return []; // no nodes
  } else if (!props.valueType?.isList) {
    return [props.modelValue as NodeReferenceData];
  } else {
    return props.modelValue as NodeReferenceData[];
  }
});
const nodes = supergraph.getManyRef(nodePtrs);

const hasValue = computed(() => {
  if (props.modelValue == null) {
    return false;
  } else if (props.valueType?.isList) {
    return (props.modelValue as any[]).length > 0;
  } else {
    return true;
  }
});
type ItemVignette = { title: string | undefined; icon: IconData | undefined; status: "found" | "pending" | "missing" };
function getItemVignette(value: any): ItemVignette {
  const item = getItemFromValue(value);
  if (item != null) {
    return { title: item?.title, icon: (item as any)?.icon, status: "found" };
  } else if (isNodeRef(value)) {
    return {
      title: undefined,
      icon: ICON_BY_NODE_TYPE[value.nodeType],
      status: autoloader.isPending(value) ? "pending" : "missing",
    };
  } else {
    return { title: undefined, icon: undefined, status: "missing" };
  }
}
const currentItems: Ref<ItemVignette[]> = computed(() => {
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  nodes.value; // 'borrow' reactivity from nodes (not great but search index isn't reactive, see above)
  if (!hasValue.value) {
    return [];
  } else if (!props.valueType?.isList) {
    return [getItemVignette(props.modelValue)];
  } else {
    return (props.modelValue as any[]).map((v) => getItemVignette(v));
  }
});

//
// Search
//

const { candidates, results, resultsTotal, isLoading, update, getItemFromValue, getValueFromItem } = useValueSearch({
  query,
  valueType: toValueRef(toRef(props, "valueType")),
  isEnabled: toRef(props, "isInline"),
});
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});

// auto-select best match when searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultId.value = results.value[0].id;
  }
});

//
// Interaction
//

function isSelected(value: SearchItem) {
  if (!props.valueType?.isList) {
    return hasValue.value && value.id == getItemFromValue(props.modelValue)?.id;
  } else {
    return (props.modelValue as any[])?.some((v) => value.id == getItemFromValue(v)?.id);
  }
}
function isActive(item: SearchItem) {
  return item.id === activeResultId.value;
}
function select(option: string | SearchItem | undefined) {
  if (typeof option == "string") option = results.value.find((r) => r.id === option);
  if (option == null) return;
  const value = getValueFromItem(option);
  if (value != null) {
    if (!props.valueType?.isList) {
      apply(value);
    } else if (!isSelected(option)) {
      apply([...((props.modelValue as any[]) ?? []), value]);
    }
    query.value = "";
  }
}
function deselect(option: SearchItem | number) {
  if (!props.valueType?.isList) {
    apply(undefined);
  } else if (hasValue.value) {
    if (typeof option == "number") {
      apply((props.modelValue as any[]).filter((v, i) => i != option));
    } else {
      apply((props.modelValue as any[]).filter((v) => getItemFromValue(v)?.id != option.id));
    }
  }
}
function apply(value: any) {
  emit("update:modelValue", value ?? undefined);
  emit("apply", value ?? undefined, props.valueType?.isList);
}
function clear() {
  apply(undefined);
}

function focus(anchor?: "previous" | "next" | FocusAnchor | NodeReferenceData) {
  if (!props.isInline) {
    return buttonRef.value;
  } else {
    const idx = results.value.findIndex((r) => r.id === activeResultId.value);
    if (anchor == "previous") {
      activeResultId.value = results.value[idx > 0 ? idx - 1 : results.value.length - 1].id;
    } else if (anchor == "next") {
      activeResultId.value = results.value[idx < results.value.length - 1 ? idx + 1 : 0].id;
    }
    if (activeResultId.value != null) {
      resultsRefs.value[activeResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
    }
    return queryRef.value;
  }
}

defineExpose<ViewExposed>({
  self,
  id,
  focus,
  interact: () => {
    if (!props.isInline) {
      buttonRef.value?.click();
    } else {
      queryRef.value?.focus();
    }
  },
});
</script>
<template>
  <ViewContentWrapper :type="ViewType.PICKER" v-bind="props">
    <!-- TODO :Incomplete: Picker.isDisabled/... -->
    <!-- Dropdown -->
    <!-- NOTE: dropdown button style should match inline combobox header style since we overlay them -->
    <div
      v-if="!props.isInline && (variant == null || variant == PickerVariant.DROPDOWN)"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: ViewType.PICKER,
          placement: 'inside-top-left',
          isEnabled: !isDisabled && isInput,
          // minimal picker has no padding, but popover picker does, so add offset to ensure it's aligned
          offset: isMinimal ? { x: -10, y: -5 } : undefined,
          referenceMargin: 0,
          dontAnimate: isMinimal,
          props: {
            ...(props as ViewProps),
            title: undefined, // clear title
            size: {
              metatype: ObjectType.RECTANGLE,
              width: Math.max(
                MIN_WIDTH,
                buttonRef?.getBoundingClientRect().width! + (isMinimal ? 10 : 0), // see above
              ),
            },
            isPopover: false, // want clean inline style so it matches this variant
            isInline: true,
          },
          onApply: (value: any) => apply(value),
        })
      "
      role="button"
      :disabled="props.isDisabled || !props.isInput"
      class="group flex w-full flex-row flex-wrap items-center gap-y-1 rounded border-gray-200 hover:border-gray-200 data-[popover=true]:border-gray-200"
      :class="[!isMinimal ? 'border px-2.5 py-1' : '']"
    >
      <!-- Current value -->
      <template v-if="hasValue">
        <button
          v-for="(v, i) in currentItems"
          :key="i"
          class="mr-2 flex flex-row items-center gap-x-1.5 rounded"
          :class="[valueType?.isList ? 'bg-gray-100 px-1' : '', v.status == 'pending' ? 'animate-pulse' : '']"
        >
          <IconInline v-if="v.icon" v-bind="v.icon" class="w-5 text-center text-gray-700" />
          <span v-if="v.status == 'pending'" class="truncate">...</span>
          <span v-else class="truncate">{{ v.title ?? "???" }}</span>
        </button>
      </template>
      <div
        v-else
        class="mr-2 transition-colors duration-150"
        :class="isMinimal ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <IconInline v-if="facetIcon" v-bind="facetIcon" class="mr-1.5 w-5 text-gray-400" />
        <span class="truncate text-gray-400">{{ facetName ?? "Select" }}</span>
      </div>
      <!-- Add -->
      <button
        v-if="!isDisabled && isInput && valueType?.isList"
        class="mr-2 text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
      >
        <i class="fas fa-plus" />
      </button>
      <!-- Controls -->
      <div
        v-if="!isDisabled && isInput"
        class="ml-auto flex-shrink-0 pl-1.5 transition-colors duration-150"
        :class="isMinimal ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <!-- Clear -->
        <button
          v-if="hasValue && !valueType?.isRequired"
          class="mr-2 text-gray-400 opacity-0 transition-colors duration-150 hover:text-gray-700 group-hover:opacity-100"
          @click.stop="clear"
        >
          <i class="fas fa-xmark" />
        </button>
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-gray-700" />
      </div>
    </div>

    <!-- Inline Multi-Toggle -->
    <div
      v-else-if="variant == PickerVariant.MULTI_TOGGLE"
      class="flex h-7 w-full flex-row items-center justify-between gap-x-1 truncate rounded bg-gray-100 px-0.5"
    >
      <!-- Inline choice -->
      <button
        v-for="item in results"
        :key="item.id"
        v-tooltip="{
          icon: (item as any).icon,
          title: item.title,
          small: true,
          group: 'picker',
        }"
        :data-selected="isSelected(item)"
        :disabled="props.isDisabled"
        class="group flex-1 flex-shrink-0 truncate rounded px-0.5 py-0.5 text-center font-medium shadow-gray-200 hover:bg-gray-200 hover:text-gray-800 enabled:text-gray-600 disabled:text-gray-400 data-[selected=true]:bg-white data-[selected=true]:text-gray-700 data-[selected=true]:shadow-sm"
        @click.prevent="!isSelected(item) || valueType?.isRequired ? select(item) : clear()"
      >
        <IconInline v-if="isMinimal && (item as any).icon" v-bind="(item as any).icon" class="w-5" />
        <span v-else class="truncate">{{ item.title }}</span>
      </button>
      <div v-if="results.length == 0" class="mx-auto">
        <i class="fas fa-empty-set mr-1.5 text-gray-600" />
        <span class="text-gray-500">No options</span>
      </div>
    </div>

    <!-- Inline Combobox -->
    <div v-else-if="isInline" :style="{ width: width + 'px' }">
      <!-- Header -->
      <div
        class="flex flex-row flex-wrap items-center gap-y-1.5 px-2.5 py-1"
        :class="[
          isPopover ? 'mx-2 mb-0.5 mt-1.5 rounded border border-gray-200 bg-gray-100' : 'border-b border-gray-200',
        ]"
      >
        <!-- Current value -->
        <template v-if="valueType?.isList">
          <button
            v-for="(v, i) in currentItems"
            :key="i"
            class="mr-2 flex flex-row items-center rounded bg-gray-100 px-1"
          >
            <IconInline v-if="v.icon" v-bind="v.icon" class="mr-1.5 w-5 text-gray-700" />
            <span class="truncate">{{ v.title ?? "???" }}</span>
            <!-- Deselect -->
            <button
              v-if="!isDisabled && isInput"
              class="ml-1.5 text-gray-400 transition-colors duration-150 hover:text-gray-700"
              @click.stop="deselect(i)"
            >
              <i class="fas fa-xmark" />
            </button>
          </button>
        </template>
        <!-- Query -->
        <span class="flex flex-row items-center">
          <i v-if="isLoading" class="fas fa-spinner-third w-5 animate-spin text-center text-gray-700" />
          <IconInline
            v-else
            v-bind="icon ?? makeIcon({ faName: 'fas fa-caret-circle-down' })"
            class="w-5 text-center text-gray-700"
          />
          <input
            ref="queryRef"
            v-model="query"
            type="text"
            class="ml-2 w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
            :placeholder="placeholder ?? `Select ${facetName ?? '???'}`"
            @keydown.enter.stop.prevent="activeResultId && select(activeResultId)"
            @keydown.up.stop.prevent="focus('previous')"
            @keydown.down.stop.prevent="focus('next')"
          />
        </span>
      </div>
      <!-- Body -->
      <Scroll
        id="body"
        size-is-dynamic
        :size="{ width, height: MAX_HEIGHT }"
        :orientation="Orientation.VERTICAL"
        :track-width="ScrollbarWidth.sm"
        track-is-overlay
      >
        <!-- Results -->
        <ul
          v-if="results.length > 0"
          class="flex max-w-full flex-col py-0.5"
          :class="[isPopover ? 'mx-2 mb-1 mt-0.5' : '']"
          :style="{ maxWidth: `${width}px` }"
        >
          <template v-for="item in results" :key="item.id">
            <!-- Results -->
            <li
              :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
              role="menuitem"
              class="mx-0.5 mb-[1px] mr-1.5 mt-[1px] flex h-[28px] max-w-full cursor-pointer flex-row items-center truncate rounded border border-transparent px-1.5 hover:bg-gray-100"
              :class="[isActive(item) ? 'bg-gray-100' : '']"
              :data-selected="isSelected(item)"
              :data-active="isActive(item)"
              @click.prevent="select(item)"
            >
              <!-- Main content -->
              <IconInline
                v-if="(item as any).icon"
                v-bind="(item as any).icon"
                class="mr-1.5 w-5 flex-shrink-0 text-gray-700"
              />
              <span v-else class="mr-1.5 w-5 flex-shrink-0 text-gray-700" />
              <span class="max-w-full select-none truncate">
                <span class="truncate" v-html="item.titleMarked ?? item.title" />
                <!-- Checked -->
                <i v-if="isSelected(item)" class="fas fa-check flex-shrink-0 pl-2 pr-1 text-gray-700" />
              </span>
              <!-- Metadata -->
              <span class="ml-auto truncate pl-2 text-right">
                <!-- Path -->
                <span
                  v-if="valueType?.kind != TypeKind.BASED_NODE && 'path' in item"
                  class="truncate pl-2 text-gray-500"
                >
                  <span v-html="item.pathMarked ?? item.path" />
                </span>
                <span
                  v-else-if="item.alias"
                  class="ml-auto truncate pl-2 text-right text-gray-500"
                  v-html="item.aliasMarked ?? item.alias"
                />
              </span>
            </li>
          </template>
        </ul>
        <!-- NOTE: Picker no results/overflow is very similar to Omnibar/Icon/etc. :ResultInfo -->
        <!-- Too many results (truncated) -->
        <div v-if="results.length < resultsTotal" class="my-1 max-w-full px-[11px] pb-2 text-gray-500">
          <i class="fas fas fa-ellipsis w-5 text-center" />
          <span class="ml-1.5">
            <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results
            <template v-if="query.length > 0">for </template>
            <span class="truncate font-semibold">{{ query }}</span>
          </span>
        </div>
        <!-- Help -->
        <div v-if="!isLoading && results.length == 0" class="max-w-full py-1">
          <div class="px-[11px] py-1 text-gray-500">
            <i class="fas fa-empty-set w-5 text-center text-gray-600" />
            <span class="ml-1">{{ resultsTotal }} results</span>
          </div>
        </div>
      </Scroll>
    </div>
  </ViewContentWrapper>
</template>
