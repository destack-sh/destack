<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { useObjectLayout } from "@/ui/object";
import { computedValue } from "@/utils/ref";
import { ModelValueOptions, ViewEmits, type ViewExpose } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
import ClaimList from "@/views/objects/ClaimList.vue";
import FieldList from "@/views/objects/FieldList.vue";
import MembershipList from "@/views/objects/MembershipList.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { toRef } from "vue";

const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Partial<
    Pick<ViewData, "icon" | "size" | "nodePtr" | "subnodePacked" | "valueType" | "isMinimal" | "isInput" | "isDisabled">
  >
>();
const modelValue = defineModel<any>("modelValue");
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

// node / layout
const nodePtr = computedValue(() => props.nodePtr);
const { node, connection, layout } = useObjectLayout({
  isInput: toRef(props, "isInput"),
  nodePtr: toRef(props, "nodePtr"),
  valueType: toRef(props, "valueType"),
  valuePacked: modelValue,
  updateValuePacked: (update: any, options: any) => {
    emit("update:modelValue", { ...modelValue.value, ...update }, options);
  },
});

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div v-if="layout" class="flex flex-col gap-y-2">
    <!-- Sections -->
    <div
      v-for="(section, i) in layout.sections.filter((s) => s.rows.length > 0)"
      :key="section.title ?? i"
      class="group/section"
    >
      <!-- Section header -->
      <div
        v-if="section.title"
        class="group/section-header relative mx-1 flex flex-row items-center rounded-sm px-3"
        :style="{ height: `${SECTION_HEADER_HEIGHT}px` }"
      >
        <!-- Title -->
        <span class="font-medium">{{ section.title }}</span>
        <!-- Subtitle -->
        <span v-if="section.subtitle" class="ml-1.5 text-gray-400">{{ section.subtitle }}</span>
        <!-- Meta -->
        <div class="ml-auto flex flex-row items-center gap-x-1 pr-1">
          <!-- Summary -->
          <span v-if="section.summary != null" class="text-gray-400">
            {{ section.summary }}
          </span>
          <!-- Commands -->
          <button
            v-for="action in section.actions"
            :key="action.title"
            v-tooltip="{ title: action.title, small: true, group: 'section.header' }"
            aria-hidden
            class="mt-1 rounded-sm px-1 text-gray-400 hover:bg-gray-100 group-hover/section-header:text-gray-700"
            @click.stop="
              (e) => {
                action.action(e);
              }
            "
          >
            <IconInline v-bind="action.icon" />
          </button>
        </div>
      </div>
      <!-- Section content -->
      <div v-if="section.rows.length > 0" class="flex flex-col gap-y-1.5">
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
            <!-- Commands -->
            <!-- ... -->
          </div>

          <!-- Body -->
          <!-- Fields -->
          <div v-if="row.type == 'fields-list'" class="w-full rounded-sm border border-gray-200 px-1 py-1">
            <FieldList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="row.toolPtr ?? nodePtr"
              :field-type="row.fieldType"
              is-minimal
            />
          </div>
          <div v-else-if="row.type == 'claims-list'" class="w-full rounded-sm border border-gray-200 px-1 py-1">
            <ClaimList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="nodePtr"
              is-minimal
            />
          </div>
          <div v-else-if="row.type == 'membership-list'" class="w-full rounded-sm border border-gray-200 px-1 py-1">
            <MembershipList
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="row.delegatePtr ?? nodePtr"
              is-minimal
            />
          </div>
          <!-- Dynamic View -->
          <component
            :is="getViewComponent(row.viewType)"
            v-else-if="
              (row.type == 'view' || row.type == 'property' || row.type == 'field') && hasViewComponent(row.viewType)
            "
            :id="i + '.' + row.viewType + '.value'"
            :class="['ml-auto shrink-0', row.isFullWidth ? '' : 'text-right']"
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
            :model-value="row.read()"
            @update:model-value="(value: any, path?: ModelValueOptions) => row.write(value, path)"
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
