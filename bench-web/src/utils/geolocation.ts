import { log } from "@/utils/log";
import { createSharedComposable } from "@vueuse/core";
import { ref, shallowRef, type Ref } from "vue";

// TypeScript interface for the complete ip-api.com schema
export interface Geolocation {
  message?: string;
  continent: string;
  continentCode: string;
  country: string;
  countryCode: string;
  region: string;
  regionName: string;
  city: string;
  district: string;
  zip: string;
  lat: number;
  lon: number;
  timezone: string;
  offset: number;
  currency: string;
  isp: string;
  org: string;
  as: string;
  asname: string;
  reverse: string;
  mobile: boolean;
  proxy: boolean;
  hosting: boolean;
  query: string;
}

async function getGeolocation(): Promise<Geolocation> {
  try {
    // NOTE :Compliance: get commercial api token for ip-api.com?
    const response = await fetch("http://ip-api.com/json/");
    const data = await response.json();

    if (data.status === "fail") {
      throw new Error(data.message || "Failed to fetch geolocation");
    }
    log.trace("geolocation.complete", data);

    return data as Geolocation;
  } catch (error) {
    log.error("geolocation.error", error);
    throw error;
  }
}

const _geolocation: Ref<Geolocation | null> = shallowRef(null);

function _useGeolocation() {
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const fetchLocation = async () => {
    loading.value = true;
    error.value = null;

    try {
      _geolocation.value = await getGeolocation();
    } catch (err) {
      error.value = err instanceof Error ? err : new Error("An unknown error occurred");
    } finally {
      loading.value = false;
    }
  };

  const refreshLocation = async () => {
    await fetchLocation();
  };

  fetchLocation();

  return {
    location: _geolocation,
    loading,
    error,
    refreshLocation,
  };
}

export const useGeolocation = createSharedComposable(_useGeolocation);
export const GEOLOCATION = useGeolocation().location;
