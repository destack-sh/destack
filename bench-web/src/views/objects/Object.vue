<script lang="ts" setup>
import { getBaseFromNode, isSourceNode, toCamelName } from "@/language/const";
import { useComputedValues } from "@/language/expression";
import { makeType, makeTypeConstraint } from "@/language/field";
import { unpackSubnode, useSubnodeProperty } from "@/language/node";
import { getCustomObjectNodeType, getCustomObjectSubtype } from "@/language/value";
import {
  ActionType,
  BenchType,
  ComputedValueData,
  NodeType,
  Orientation,
  TypeData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph, useExistingConnection } from "@/system/connection";
import { canvas, pkgConnection } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { ObjectSection, makeObjectLayout } from "@/ui/object";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
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

// node
const nodePtr = computedValue(() => props.nodePtr);
const { node, connection } = supergraph.getLinkRef(nodePtr);
const { graph } = useExistingConnection(nodePtr);
const fields = graph.getChildrenRef(node, NodeType.FIELD);

// delegate (base or some other delegate)
const delegatePtr = computed(
  () => {
    if (props.valueType?.baseTypePtr != null) return props.valueType.baseTypePtr;
    else if (isNode(node.value, NodeType.ACTION) && node.value.type == ActionType.TOOL) return node.value.toolPtr;
    else if (node.value != null) return getBaseFromNode(node.value);
    else return null;
  },
);
const { graph: delegateGraph, connection: delegateConnection } = useExistingConnection(delegatePtr);
const delegate = delegateGraph.getRef(delegatePtr);
const delegateFields = delegateGraph.getChildrenRef(delegate, NodeType.FIELD);

// computed
const computer = useComputedValues({
  computedValues: computed(() => (isSourceNode(node.value) ? node.value.computedValues : [])),
  update: (computedValues) => {
    if (isSourceNode(node.value)) {
      connection.value?.tx.update(node.value, { computedValues });
    }
  },
});
const computedType = computed<TypeData | undefined>(() => {
  if (props.computedType != null) return props.computedType;
  if (node.value == null) return undefined;
  const type = makeType({
    benchType: BenchType.COMPUTED_VALUE,
    isRequired: true,
    constraint: makeTypeConstraint({ nodeScopePtr: [node.value.parentPtr!] }), // NOTE: should really be the containing runnable
  });
  return type;
});

// layout
const kind = computed(() => {
  if (props.valueType?.kind == TypeKind.PARTIAL_OBJECT) return "partial";
  else if (props.valueType?.kind == TypeKind.CUSTOM_OBJECT) return "custom";
  else return "node";
});
const layout = computed(() => {
  if (kind.value == "node") {
    if (node.value == null) return null;
    const nodeType = node.value.metatype as unknown as NodeType;
    const subtype = (node as any).type;
    const subnode =
      (node.value.subnodePacked as any)?.[subtype?.toString()!] != null
        ? (unpackSubnode(nodeType, subtype as never, node.value.subnodePacked) as any)
        : null;
    return makeObjectLayout({
      kind: "node",
      node: node.value,
      fields: fields.value,
      nodeType: node.value.metatype,
      subtype: subtype,
      subnode: subnode,
      delegate: delegate.value,
      delegateFields: delegateFields.value,
      graph,
      update: (update, options) => {
        if (node.value != null) {
          connection.value?.tx.update(node.value, update, options);
        }
      },
      txFactory: () => (connection.value ?? pkgConnection).tx,
    });
  } else if (kind.value == "partial") {
    if (props.valueType == null) return null;
    const nodeType = getCustomObjectNodeType(props.valueType, modelValue.value) as NodeType | null;
    const subtype = getCustomObjectSubtype(props.valueType, modelValue.value);
    return makeObjectLayout({
      kind: "partial",
      nodeType,
      subtype,
      node: null, // nocheckin,
      subnode: null, // nocheckin,
      valueType: props.valueType,
      valuePacked: modelValue.value,
      computedType: computedType.value,
      fields: fields.value,
      delegate: delegate.value,
      delegateFields: delegateFields.value,
      graph,
      update: (update, options) => {
        emit("update:modelValue", { ...modelValue.value, ...update });
      },
      txFactory: () => (connection.value ?? pkgConnection).tx,
    });
  } else if (kind.value == "custom") {
    return makeObjectLayout({
      kind: "custom",
      valueType: props.valueType!,
      valuePacked: modelValue.value,
      computedType: computedType.value,
      fields: fields.value,
      delegate: delegate.value,
      delegateFields: delegateFields.value,
      graph,
      update: (update, options) => {
        emit("update:modelValue", { ...modelValue.value, ...update });
      },
      txFactory: () => (connection.value ?? pkgConnection).tx,
    });
  }
  return null;
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
            row.type == 'view' || row.type == 'property' || row.type == 'object' || row.type == 'field'
              ? row.isFullWidth ||
                ((row.type == 'property' || row.type == 'field') && computer.has(row.computedPathKey))
                ? 'flex flex-col gap-y-1'
                : 'flex flex-row flex-wrap items-center gap-x-1'
              : '',
          ]"
        >
          <!-- Header -->
          <div v-if="row.title" class="flex flex-1 flex-row items-center py-0.5">
            <!-- Title -->
            <span v-if="row.type != 'field'" class="">{{ row.title }}</span>
            <Field v-else :id="i + '.value'" :node-ptr="toNodeRef(row.field)" is-minimal />
            <span v-if="row.subtitle" class="ml-1.5 text-gray-400">{{ row.subtitle }}</span>
            <!-- Actions -->
            <div class="ml-auto pr-1">
              <!-- Computed actions :ComputedView -->
              <button
                v-if="(row.type == 'property' || row.type == 'field') && row.isComputable"
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
            v-bind="row.viewProps as any"
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
