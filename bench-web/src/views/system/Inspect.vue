<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { NodeType, Orientation, Variant, ViewData } from "@/proto/wire";
import { toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { makeInspectLayout } from "@/ui/inspect";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import Type from "@/views/system/Type.vue";
import { computed, toRef } from "vue";

const SECTION_HEADER_HEIGHT = 32;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "icon" | "size" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr) ?? inspectionPtr.value);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(nodePtr);
const node = pkgGraph.getRef(nodePtr);
const layout = computed(() => (node.value != null ? makeInspectLayout(node.value) : null));

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="node && layout" class="flex flex-col gap-y-4">
    <!-- Sections -->
    <div v-for="(section, i) in layout.sections" :key="section.title ?? i">
      <!-- Section header -->
      <div
        v-if="section.title"
        class="mx-4 flex flex-row items-center"
        :style="{ height: `${SECTION_HEADER_HEIGHT}px` }"
      >
        <span class="font-medium">{{ section.title }}</span>
      </div>
      <!-- Section content -->
      <div class="flex flex-col gap-y-1.5 py-1">
        <!-- Row -->
        <div
          v-for="(row, i) in section.rows"
          :key="row.title ?? i"
          class="mx-4"
          :class="[
            row.type == 'property'
              ? row.isFullWidth
                ? 'flex flex-col gap-y-0.5'
                : 'flex flex-row flex-wrap items-center gap-x-[10%]'
              : '',
          ]"
        >
          <div v-if="row.title" class="py-0.5">
            <span class="">{{ row.title }}</span>
          </div>
          <!-- Fields -->
          <div v-if="row.type == 'fields'" class="rounded border border-gray-200">
            <Type
              :id="row.title ?? `type-${i}`"
              :orientation="Orientation.VERTICAL"
              :node-ptr="toNodeRefOneOf(nodePtr!)"
              :field-type="row.fieldType"
              :variant="Variant.STEALTH"
            />
          </div>
          <!-- Property -->
          <component
            :is="getViewComponent(row.viewType)"
            v-if="row.type == 'property' && hasViewComponent(row.viewType)"
            :id="i + '.value'"
            :class="['ml-auto flex-shrink-0', row.isFullWidth ? '' : 'text-right']"
            :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)' }"
            v-bind="{ ...row.viewProps, isInput: row.isInput }"
            :model-value="row.read()"
            @update:model-value="(value: any) => row.write(pkgConnection.tx, pkgGraph, value)"
          />
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
