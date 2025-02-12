<script lang="ts" setup>
import { isSourceNode, toCamelName } from "@/language/core/const";
import { useSubnodeProperty } from "@/language/core/node";
import { ComputedValueData, NodeType, Orientation, PathData, TypeData, ViewData, ViewType } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { ObjectSection, useObjectLayout } from "@/ui/object";
import { computedValue } from "@/utils/ref";
import { ModelValueOptions, ViewEmits, type ViewExpose } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
import ComputedValue from "@/views/objects/ComputedValue.vue";
import FieldList from "@/views/objects/FieldList.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { toRef } from "vue";

const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    isComputable?: boolean;
    computedType?: TypeData;
    computedPrefix?: PathData;
  } & Partial<
    Pick<ViewData, "icon" | "size" | "nodePtr" | "subnodePacked" | "valueType" | "isMinimal" | "isInput" | "isDisabled">
  >
>();
const modelValue = defineModel<any>("modelValue");
const computedValues = defineModel<ComputedValueData[] | undefined>("computedValues");
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

// node / layout
const nodePtr = computedValue(() => props.nodePtr);
const { node, connection, layout, computer, computedType } = useObjectLayout({
  isInput: toRef(props, "isInput"),
  nodePtr: toRef(props, "nodePtr"),
  valueType: toRef(props, "valueType"),
  valuePacked: modelValue,
  computedPrefix: toRef(props, "computedPrefix"),
  computedValues,
  computedType: toRef(props, "computedType"),
  updateValuePacked: (update: any, options: any) => {
    emit("update:modelValue", { ...modelValue.value, ...update }, options);
  },
  updateComputedValues: (computedValues: ComputedValueData[] | undefined) => {
    emit("update:computedValues", computedValues);
  },
});

// sections
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
      newExpandedSections = expandedSections.value.filter((key) => section.key && key != section.key);
    } else {
      newExpandedSections = [...(expandedSections.value ?? []), section.key!];
    }
    state.update(
      { metatype: NodeType.VIEW, type: ViewType.OBJECT, subnode: { expandedSections: newExpandedSections } },
      { debounce: "long" },
    );
  } else {
    let newCollapsedSections;
    if (isSectionExpanded(section)) {
      newCollapsedSections = [...(collapsedSections.value ?? []), section.key!];
    } else {
      newCollapsedSections = collapsedSections.value.filter((key) => section.key && key != section.key);
    }
    state.update(
      { metatype: NodeType.VIEW, type: ViewType.OBJECT, subnode: { collapsedSections: newCollapsedSections } },
      { debounce: "long" },
    );
  }
}

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div v-if="layout" class="flex flex-col gap-y-2">
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
            aria-hidden
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
      <div v-if="section.rows.length > 0 && isSectionExpanded(section)" class="flex flex-col gap-y-1.5">
        <!-- Row -->
        <div
          v-for="(row, i) in section.rows"
          :key="row.title ?? i"
          class=""
          :class="[
            !isMinimal ? 'mx-4' : '',
            (row as any).isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row items-start gap-x-1 gap-y-1',
          ]"
        >
          <!-- Header -->
          <div v-if="row.title" class="flex flex-1 flex-row items-center pt-1">
            <!-- Title -->
            <span v-if="row.type != 'field'" class="">{{ row.title }}</span>
            <Field v-else :id="i + '.value'" :node-ptr="toNodeRef(row.field)" is-minimal />
            <span v-if="row.subtitle" class="ml-1.5 text-gray-400">{{ row.subtitle }}</span>
            <!-- Actions -->
            <div class="ml-auto pr-1">
              <!-- Computed actions -->
              <button
                v-if="(row.type == 'property' || row.type == 'field') && row.isComputable"
                v-tooltip="{ title: `Set ${row.title} dynamically`, small: true, group: 'section.header' }"
                class="rounded px-0.5 transition-colors duration-75"
                :class="
                  computer.has(row.computedPathKey)
                    ? 'text-primary-700 hover:bg-gray-100'
                    : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700'
                "
                @click="() => computer.toggle(row.computedPath!)"
              >
                <i class="fas fa-percent" />
              </button>
            </div>
          </div>

          <!-- Body -->
          <!-- Fields -->
          <div v-if="row.type == 'fields-list'" class="w-full rounded border border-gray-200">
            <FieldList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="row.toolPtr ?? nodePtr"
              :field-type="row.fieldType"
              is-minimal
            />
          </div>
          <!-- Computed View -->
          <ComputedValue
            v-else-if="(row.type == 'property' || row.type == 'field') && computer.has(row.computedPathKey)"
            :id="i + '.computed.value'"
            class="w-full"
            :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)', minHeight: ROW_HEIGHT_MIN + 'px' }"
            is-input
            :model-value="computer.get(row.computedPathKey)"
            :value-type="computedType"
            @update:model-value="(value: any) => computer.set(row.computedPath!, value)"
          />
          <!-- Dynamic View -->
          <component
            :is="getViewComponent(row.viewType)"
            v-else-if="
              (row.type == 'view' || row.type == 'property' || row.type == 'field') && hasViewComponent(row.viewType)
            "
            :id="i + '.' + row.viewType + '.value'"
            :class="['ml-auto flex-shrink-0', row.isFullWidth ? '' : 'text-right']"
            :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)', minHeight: ROW_HEIGHT_MIN + 'px' }"
            v-bind="row.viewProps"
            :model-value="row.read()"
            @update:model-value="
              (value: any) => {
                row.write(value, row.type == 'field' ? row.options : undefined);
              }
            "
          />
          <!-- Object -->
          <Object
            v-else-if="row.type == 'object'"
            :id="i + '.object.value'"
            class=""
            :style="{ width: '100%', minHeight: ROW_HEIGHT_MIN + 'px' }"
            v-bind="row.viewProps as any"
            :is-computable="row.isComputable"
            :computed-values="isSourceNode(node) ? node.computedValues : undefined"
            :computed-prefix="row.computedPath"
            :computed-type="computedType"
            :model-value="row.read()"
            @update:model-value="(value: any, path?: ModelValueOptions) => row.write(value, path)"
            @update:computed-values="
              (computedValues: ComputedValueData[] | undefined) => {
                if (isSourceNode(node) && layout?.kind != 'partial') {
                  connection?.tx.update(node, { computedValues });
                } else {
                  emit('update:computedValues', computedValues);
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
    <div v-if="node != null && layout.sections.length == 0" class="mx-4">
      <!-- Empty state -->
      <span class="text-gray-400">Just a {{ toCamelName(NodeType, node.metatype) }}.</span>
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty/missing state -->
  </div>
</template>
