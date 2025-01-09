<script lang="ts" setup>
import { isSourceNode, toCamelName } from "@/language/const";
import { useComputedValues } from "@/language/expression";
import { makeTypeConstraint, makeType } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import {
  BenchType,
  ComputedValueData,
  NodeType,
  Orientation,
  TypeData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph, useExistingConnection } from "@/system/connection";
import { canvas, pkgConnection } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { ObjectSection, makeObjectLayout } from "@/ui/object";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import ComputedValue from "@/views/objects/ComputedValue.vue";
import FieldList from "@/views/objects/FieldList.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef } from "vue";

const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    computedType?: TypeData;
  } & Pick<ViewData, "icon" | "size" | "nodePtr" | "subnodePacked" | "valueType">
>();
const modelValue = defineModel<any>("modelValue");
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

const kind = computed(() => {
  if (props.valueType?.kind == TypeKind.PARTIAL_OBJECT) return "partial";
  else if (props.valueType?.kind == TypeKind.CUSTOM_OBJECT) return "custom";
  else return "node";
});
const nodePtr = computedValue(() => props.nodePtr);
const { node, graph, connection } = supergraph.getLinkRef(nodePtr);
const basePtr = computed(
  () => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.ACTION> | undefined,
);
const { graph: baseGraph, connection: baseConnection } = useExistingConnection(basePtr);
const base = baseGraph.getRef(basePtr);
const baseFields = baseGraph.getChildrenRef(base, NodeType.FIELD);
const computer = useComputedValues({
  computedValues: computed(() => (isSourceNode(node.value) ? node.value.computedValues : [])),
  update: (computedValues) => {
    if (isSourceNode(node.value)) {
      connection.value?.tx.update(node.value, { computedValues });
    }
  },
});
const computedType = computed<TypeData | undefined>(() => {
  if (node.value == null) return undefined;
  const type = makeType({
    benchType: BenchType.COMPUTED_VALUE,
    isRequired: true,
    constraint: makeTypeConstraint({ nodeScopePtr: [node.value.parentPtr!] }), // NOTE: should really be the containing runnable
  });
  return type;
});
const layout = computed(() => {
  if (node.value == null) return null;
  const layout = makeObjectLayout({
    kind: kind.value,
    node: node.value,
    base: base.value,
    baseFields: baseFields.value,
    graph: graph.value!,
    txFactory: () => (connection.value ?? pkgConnection).tx,
    computedType: computedType.value,
  });
  return layout;
});

const isLayoutEmpty = computed(() => !layout.value?.sections.some((section) => section.rows.length > 0));
const expandedSections = useSubnodeProperty(NodeType.VIEW, ViewType.OBJECT, subnodePacked, "expandedSections");
const collapsedSections = useSubnodeProperty(NodeType.VIEW, ViewType.OBJECT, subnodePacked, "collapsedSections");
function isSectionExpanded(section: ObjectSection) {
  if (section.key == null) return true;
  if (section.isDefaultCollapsed) return expandedSections.value?.includes(section.key);
  else return !collapsedSections.value?.includes(section.key);
}
function toggleSection(section: ObjectSection) {
  if (section.title == null) throw new Error("cannot toggle a section without a title");
  if (section.isDefaultCollapsed) {
    let newExpandedSections;
    if (isSectionExpanded(section)) {
      newExpandedSections = expandedSections.value.filter((key) => key != section.key);
    } else {
      newExpandedSections = [...(expandedSections.value ?? []), section.key];
    }
    state.update(
      { metatype: NodeType.VIEW, type: ViewType.OBJECT, subnode: { expandedSections: newExpandedSections } },
      { debounce: "long" },
    );
  } else {
    let newCollapsedSections;
    if (isSectionExpanded(section)) {
      newCollapsedSections = [...(collapsedSections.value ?? []), section.key];
    } else {
      newCollapsedSections = collapsedSections.value.filter((key) => key != section.key);
    }
    state.update(
      { metatype: NodeType.VIEW, type: ViewType.OBJECT, subnode: { collapsedSections: newCollapsedSections } },
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
          class="group/row mx-4"
          :class="[
            row.type == 'view' || row.type == 'property' || row.type == 'object'
              ? row.isFullWidth || (row.type == 'property' && computer.has(row.computedPathKey))
                ? 'flex flex-col gap-y-1'
                : 'flex flex-row flex-wrap items-center gap-x-1'
              : '',
          ]"
        >
          <!-- Header -->
          <div v-if="row.title" class="flex flex-1 flex-row items-center py-0.5">
            <span class="">{{ row.title }}</span>
            <span v-if="row.subtitle" class="ml-1.5 text-gray-400">{{ row.subtitle }}</span>
            <!-- Actions -->
            <div class="ml-auto pr-1">
              <!-- Computed actions :ComputedView -->
              <button
                v-if="row.type == 'property' && row.isComputable"
                v-tooltip="{ title: `Set ${row.title} dynamically`, small: true, group: 'section.header' }"
                class="rounded px-0.5 transition-colors duration-75"
                :class="
                  computer.has(row.computedPathKey)
                    ? 'text-primary-700 hover:bg-gray-100'
                    : 'text-gray-400 hover/row:bg-gray-100 group-hover/row:text-gray-700'
                "
                @click="() => computer.toggle(row.computedPath!)"
              >
                <i class="fas fa-percent" />
              </button>
            </div>
          </div>

          <!-- Body -->
          <!-- Fields -->
          <div v-if="row.type == 'fields-list'" class="rounded border border-gray-200">
            <FieldList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="row.toolPtr ?? nodePtr"
              :field-type="row.fieldType"
              is-minimal
            />
          </div>
          <!-- Computed View :ComputedView -->
          <ComputedValue
            v-else-if="(row.type == 'property' || row.type == 'field') && computer.has(row.computedPathKey)"
            :id="i + '.value'"
            class="w-full"
            is-input
            :model-value="computer.get(row.computedPathKey)"
            :value-type="computedType"
            @update:model-value="(value) => computer.set(row.computedPath!, value)"
          />
          <!-- Dynamic View -->
          <component
            :is="getViewComponent(row.viewType)"
            v-else-if="
              (row.type == 'view' || row.type == 'property' || row.type == 'field') && hasViewComponent(row.viewType)
            "
            :id="i + '.value'"
            :class="['ml-auto flex-shrink-0', row.isFullWidth ? '' : 'text-right']"
            :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)', minHeight: ROW_HEIGHT_MIN + 'px' }"
            v-bind="row.viewProps"
            :is-computable="(row.type == 'property' || row.type == 'field') && row.isComputable"
            :model-value="row.read()"
            @update:model-value="(value: any, path?: any) => row.write(value, path)"
          />
          <!-- Object -->
          <Object
            v-else-if="row.type == 'object'"
            :id="i + '.value'"
            :class="['ml-auto flex-shrink-0']"
            :style="{ width: '100%', minHeight: ROW_HEIGHT_MIN + 'px' }"
            v-bind="(row.viewProps as any)"
            :is-computable="row.isComputable"
            :computed-values="isSourceNode(node) ? node.computedValues : undefined"
            :computed-prefix="row.computedPath"
            :computed-type="computedType"
            :model-value="row.read()"
            @update:model-value="(value: any, path?: any) => row.write(value, path)"
            @update:computed-values="
              (computedValues: ComputedValueData[]) => {
                if (isSourceNode(node)) {
                  connection?.tx.update(node, { computedValues });
                }
              }
            "
          />
          <!-- Icon -->
          <div v-else-if="row.type == 'icon'" class="w-full text-center">
            <IconInline class="text-gray-400" v-bind="row.icon" />
          </div>
          <!-- Text -->
          <div v-else-if="row.type == 'text'" class="text-gray-400">
            <span>{{ row.text }}</span>
          </div>
          <!-- Line -->
          <div v-else-if="row.type == 'line'" class="relative my-0.5 h-px w-full bg-gray-200">
            <span
              v-if="row.text"
              class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-white px-2 text-xs text-gray-400"
              >{{ row.text }}</span
            >
          </div>
          <!-- Error -->
          <div v-else class="text-red-500">
            <span>No View for '{{ row.type }}'</span>
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
