<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { useSubnodeProperty } from "@/language/node";
import { NodeType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { canvas, pkgConnection } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { DetailSection, makeInspectLayout } from "@/ui/detail";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import FieldList from "@/views/structs/FieldList.vue";
import { computed, toRef } from "vue";

const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "icon" | "size" | "nodePtr" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = computedValue(() => props.nodePtr);
const { node, graph, connection } = supergraph.getLinkRef(nodePtr);
const layout = computed(() => {
  if (node.value == null) return null;
  const layout = makeInspectLayout(node.value, graph.value!, () => (connection.value ?? pkgConnection).tx);
  return layout;
});

const expandedSections = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.DETAIL,
  toRef(props, "subnodePacked"),
  "expandedSections",
);
const collapsedSections = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.DETAIL,
  toRef(props, "subnodePacked"),
  "collapsedSections",
);
function isSectionExpanded(section: DetailSection) {
  if (section.title == null) return true;
  if (section.isDefaultCollapsed) return expandedSections.value?.includes(section.title);
  else return !collapsedSections.value?.includes(section.title);
}
function toggleSection(section: DetailSection) {
  if (section.title == null) throw new Error("cannot toggle a section without a title");
  if (section.isDefaultCollapsed) {
    let newExpandedSections;
    if (isSectionExpanded(section)) {
      newExpandedSections = expandedSections.value.filter((title) => title != section.title);
    } else {
      newExpandedSections = [...(expandedSections.value ?? []), section.title];
    }
    state.update(
      {
        metatype: NodeType.VIEW,
        type: ViewType.DETAIL,
        subnode: { expandedSections: newExpandedSections },
      },
      { debounce: "long" },
    );
  } else {
    let newCollapsedSections;
    if (isSectionExpanded(section)) {
      newCollapsedSections = [...(collapsedSections.value ?? []), section.title];
    } else {
      newCollapsedSections = collapsedSections.value.filter((title) => title != section.title);
    }
    state.update(
      {
        metatype: NodeType.VIEW,
        type: ViewType.DETAIL,
        subnode: { collapsedSections: newCollapsedSections },
      },
      { debounce: "long" },
    );
  }
}

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="node && layout" class="flex flex-col gap-y-2">
    <!-- Sections -->
    <div v-for="(section, i) in layout.sections" :key="section.title ?? i" class="group/section">
      <!-- Section header -->
      <div
        v-if="section.title"
        class="group/section-header relative mx-1 flex flex-row items-center rounded px-3"
        :style="{ height: `${SECTION_HEADER_HEIGHT}px` }"
        @click="toggleSection(section)"
      >
        <!-- Expand/collapse indicator -->
        <i
          class="fas fa-chevron-right absolute left-0 text-xs text-gray-400 opacity-0 transition-all duration-75 group-hover/section-header:text-gray-700 group-hover/section:opacity-100"
          :class="isSectionExpanded(section) ? 'rotate-90' : 'rotate-0'"
        />
        <!-- Title -->
        <span class="font-medium">{{ section.title }}</span>
        <!-- Subtitle -->
        <span v-if="section.subtitle" class="ml-1.5 text-gray-400">{{ section.subtitle }}</span>
        <!-- Meta -->
        <div class="ml-auto flex flex-row items-center gap-x-1 pr-1">
          <!-- Summary -->
          <span v-if="!isSectionExpanded(section) && section.summary != null" class="text-gray-400">
            {{ section.summary }}
          </span>
          <!-- Actions -->
          <button
            v-for="action in section.actions"
            :key="action.title"
            v-tooltip="{ title: action.title, small: true, group: 'section.header' }"
            class="mt-1 rounded px-1 text-gray-400 hover:bg-gray-100 group-hover/section-header:text-gray-700"
            @click.stop="
              (e) => {
                action.action(e);
                if (!isSectionExpanded(section)) toggleSection(section);
              }
            "
          >
            <IconInline v-bind="action.icon" />
          </button>
        </div>
      </div>
      <!-- Section content -->
      <div v-if="section.rows.length > 0 && isSectionExpanded(section)" class="flex flex-col gap-y-1.5 py-1">
        <!-- Row -->
        <div
          v-for="(row, i) in section.rows"
          :key="row.title ?? i"
          class="mx-4"
          :class="[
            row.type == 'view'
              ? row.isFullWidth
                ? 'flex flex-col gap-y-0.5'
                : 'flex flex-row flex-wrap items-center gap-x-[5%]'
              : '',
          ]"
        >
          <!-- Header -->
          <div v-if="row.title" class="py-0.5">
            <span class="">{{ row.title }}</span>
            <span v-if="row.subtitle" class="ml-1.5 text-gray-400">{{ row.subtitle }}</span>
          </div>
          <!-- Fields -->
          <div v-if="row.type == 'fields'" class="rounded border border-gray-200">
            <FieldList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="row.delegatePtr ?? nodePtr"
              :field-type="row.fieldType"
              is-minimal
            />
          </div>
          <!-- View -->
          <component
            :is="getViewComponent(row.viewType)"
            v-else-if="row.type == 'view' && hasViewComponent(row.viewType)"
            :id="i + '.value'"
            :class="['ml-auto flex-shrink-0', row.isFullWidth ? '' : 'text-right']"
            :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)', minHeight: ROW_HEIGHT_MIN + 'px' }"
            v-bind="row.viewProps"
            :model-value="row.read()"
            @update:model-value="(value: any, path?: any) => row.write(value, path)"
          />
          <!-- Icon -->
          <div v-else-if="row.type == 'icon'" class="w-full text-center">
            <IconInline class="text-gray-400" v-bind="row.icon" />
          </div>
          <!-- Text -->
          <div v-else-if="row.type == 'text'" class="text-gray-400">
            <span>{{ row.text }}</span>
          </div>
        </div>
      </div>
    </div>
    <div v-if="layout.sections.length == 0" class="mx-4">
      <!-- Empty state -->
      <span class="text-gray-400">Just a {{ toCamelName(NodeType, node.metatype) }}.</span>
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty/missing state -->
  </div>
</template>
