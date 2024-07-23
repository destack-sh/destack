<script lang="ts" setup>
import { Orientation, RegionArea } from "@/proto/wire";
import { isDeveloperMode } from "@/system/client";
import { connections, hasPendingConnections } from "@/system/connection";
import { ICON_BY_REGION_AREA, IconInline, makeIcon } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { GEOLOCATION } from "@/utils/geolocation";
import { IS_DEV } from "@/utils/globals";
import { ScrollbarWidth } from "@/utils/layout";
import Popover from "@/views/builtins/Popover.vue";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, type Ref } from "vue";

const geolocationIcon = computed(() => (GEOLOCATION.value?.area ? ICON_BY_REGION_AREA[GEOLOCATION.value?.area] : null));

const expandedConnectionId: Ref<number | null> = ref(null);
</script>
<template>
  <!-- Connection -->
  <Popover placement="bottom" :reference-margin="8" :container-margin="4">
    <template #trigger="{ toggle }">
      <!-- Current status -->
      <button
        class="select-none rounded border-2 px-1 py-1 transition-colors"
        :class="
          connections.some((c) => c.isPaused.value || c.txBuffer.isPaused.value)
            ? 'border-warning-600'
            : 'border-transparent'
        "
        @click.stop="toggle"
      >
        <IconInline
          v-bind="geolocationIcon ?? makeIcon({ faName: 'fas fa-cloud' })"
          :class="
            !hasPendingConnections
              ? ' text-gray-700 hover:text-primary-900'
              : ' animate-pulse text-warning-600 hover:text-warning-500'
          "
        />
      </button>
    </template>
    <template #content="{ close }">
      <!-- Individual connections -->
      <!-- (will probably move this to a Connections View (maybe keep summary on hover)) -->
      <div v-outside.click.stop="close" class="p z-50 rounded border border-gray-300 bg-white text-gray-900">
        <!-- Header -->
        <div class="my-1 flex flex-row border-b border-gray-300 px-3 py-1">
          <span class="font-semibold">Connections</span>
          <div class="ml-auto">
            <span v-if="GEOLOCATION?.area" class="text-gray-400"
              >{{ toCamelName(RegionArea, GEOLOCATION?.area) }} / {{ GEOLOCATION?.detail?.city }}</span
            >
          </div>
        </div>
        <!-- Body -->
        <Scroll
          :size="{ width: 480, height: 600 }"
          size-is-dynamic
          :orientation="Orientation.VERTICAL"
          :track-width="ScrollbarWidth.sm"
        >
          <ul class="my-1.5 flex min-w-[480px] flex-col gap-y-1 px-3">
            <li v-for="connection in connections" :key="connection.id" class="">
              <!-- Header -->
              <div class="flex flex-row py-1">
                <!-- Expand/collapse -->
                <button
                  v-if="isDeveloperMode"
                  class="mr-2 text-gray-400 hover:text-primary-900"
                  @click="expandedConnectionId = expandedConnectionId == connection.id ? null : connection.id"
                >
                  <i
                    :class="[
                      'fa fa-chevron-right transition-transform duration-75',
                      expandedConnectionId == connection.id ? 'rotate-90' : '',
                    ]"
                  />
                </button>
                <!-- Metadata -->
                <span class="h-fit rounded bg-secondary-100 px-2 font-semibold text-secondary-900">
                  {{ connection.kind }}
                </span>
                <span class="ml-2 truncate font-semibold">{{ connection.name }}</span>
                <span class="ml-2 text-gray-500">#{{ connection.id }}</span>
                <!-- Status -->
                <span class="ml-auto flex flex-row pl-4 align-top">
                  <template v-if="isDeveloperMode">
                    <span class="mr-2" :class="connection.referenceCount > 0 ? '' : 'text-gray-500'">
                      {{ connection.referenceCount }}
                    </span>
                    <span class="mr-2">
                      {{ connection.epoch }}
                    </span>
                  </template>
                  <!-- Connected (status) -->
                  <span class="rounded px-1">
                    <i
                      class="fas"
                      :class="
                        connection.isConnected.value
                          ? 'fa-check text-success-600'
                          : 'fa-exclamation-circle text-warning-600'
                      "
                    />
                  </span>
                  <template v-if="isDeveloperMode">
                    <!-- Down (status & toggle) -->
                    <button class="rounded px-1 hover:bg-primary-200" @click="connection.togglePaused()">
                      <i
                        :class="
                          connection.isConnecting.value
                            ? 'fas fa-spinner-third animate-spin text-gray-500'
                            : connection.isLive && !connection.isPaused.value
                              ? 'fas fa-down text-success-600'
                              : 'fas fa-down text-secondary-500'
                        "
                      />
                    </button>
                    <!-- Up (toggle) -->
                    <button class="rounded px-1 hover:bg-primary-200" @click="connection.txBuffer.togglePaused()">
                      <i
                        class="fas fa-up"
                        :class="connection.txBuffer.isPaused.value ? 'text-secondary-500' : 'text-success-600'"
                      />
                    </button>
                  </template>
                </span>
              </div>
              <div
                v-if="connection.id == expandedConnectionId"
                class="max-h-60 overflow-y-auto rounded-md border border-gray-200 bg-gray-50 p-1 py-0.5"
              >
                {{ connection.params }}
              </div>
            </li>
          </ul>
        </Scroll>
      </div>
    </template>
  </Popover>
</template>
