/* eslint-disable no-console */
import { RegionArea, RegionContinent, Region } from "@/proto/wire";
import { IP_API_KEY } from "@/utils/globals";
import { log } from "@/utils/log";
import { formatDuration } from "@/utils/time";
import { createSharedComposable } from "@vueuse/core";
import { ref, shallowRef, type Ref } from "vue";

// see ip-api.com/docs
export interface GeolocationRaw {
  continent: string;
  continentCode: string;
  country: string;
  countryCode: string;
  countryCode3: string;
  region: string;
  regionName: string;
  city: string;
  lat: number;
  lon: number;
  isp: string;
  org: string;
  proxy: boolean;
  hosting: boolean;
  query: string;
}
const GEOLOCATION_FIELDS = [
  "status",
  "message",
  "continent",
  "continentCode",
  "country",
  "countryCode",
  "countryCode3",
  "region",
  "regionName",
  "city",
  "lat",
  "lon",
  "proxy",
  "hosting",
  "query",
];

export interface Geolocation {
  continent: RegionContinent | null;
  area: RegionArea | null;
  country: string | null;
  detail: GeolocationRaw;
}

// TODO :Security: move ip-api lookup behind proxy (to avoid exposing our token)
async function getGeolocation(): Promise<Geolocation> {
  try {
    const startedAt = performance.now();
    const response = IP_API_KEY
      ? await fetch(`https://pro.ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}&key=${IP_API_KEY}`, {
          credentials: "omit",
        })
      : await fetch(`http://ip-api.com/json/?fields=${GEOLOCATION_FIELDS.join(",")}`); // only works in HTTP context (can't request HTTP in HTTPS)
    const data = await response.json();
    if (data.status === "fail") {
      throw new Error(data.message || "Failed to fetch geolocation");
    }

    const duration = performance.now() - startedAt;
    const geolocation = parseGeolocation(data);
    log.debug("geolocation.complete", formatDuration(duration), data);
    console.group(`%cgeo`, "color:yellow");
    console.info(
      `%cCONTINENT: ${geolocation.continent ? RegionContinent[geolocation.continent] : "<unknown>"}`,
      "color:yellow",
    );
    console.info(`%cAREA: ${geolocation.area ? RegionArea[geolocation.area] : "<unknown>"}`, "color:yellow");
    console.info(`%cCOUNTRY: ${geolocation.country}`, "color:yellow");
    console.groupEnd();
    return geolocation;
  } catch (error) {
    log.error("geolocation.error", error);
    throw error;
  }
}

function parseGeolocation(data: GeolocationRaw): Geolocation {
  const continent = CONTINENT_BY_CONTINENT_CODE[data.continentCode];
  const area = AREA_BY_COUNTRY_CODE[data.countryCode];
  const gelocation: Geolocation = { continent: continent, area: area, country: data.countryCode, detail: data };
  return gelocation;
}

const CONTINENT_BY_CONTINENT_CODE: Record<string, RegionContinent> = {
  EU: RegionContinent.EUROPE,
  NA: RegionContinent.NORTH_AMERICA,
  SA: RegionContinent.SOUTH_AMERICA,
  AS: RegionContinent.ASIA,
  AF: RegionContinent.AFRICA,
  OC: RegionContinent.AUSTRALIA,
  AU: RegionContinent.AUSTRALIA,
};

const AREA_BY_COUNTRY_CODE: Record<string, RegionArea> = {
  // eu-central
  AT: RegionArea.EUROPE_CENTRAL,
  BE: RegionArea.EUROPE_CENTRAL,
  CH: RegionArea.EUROPE_CENTRAL,
  CZ: RegionArea.EUROPE_CENTRAL,
  DE: RegionArea.EUROPE_CENTRAL,
  DK: RegionArea.EUROPE_CENTRAL,
  ES: RegionArea.EUROPE_CENTRAL,
  FI: RegionArea.EUROPE_CENTRAL,
  FR: RegionArea.EUROPE_CENTRAL,
  GB: RegionArea.EUROPE_CENTRAL,
  IE: RegionArea.EUROPE_CENTRAL,
  IT: RegionArea.EUROPE_CENTRAL,
  NL: RegionArea.EUROPE_CENTRAL,
  NO: RegionArea.EUROPE_CENTRAL,
  PL: RegionArea.EUROPE_CENTRAL,
  PT: RegionArea.EUROPE_CENTRAL,
  SE: RegionArea.EUROPE_CENTRAL,
  // na-east
  US: RegionArea.NORTH_AMERICA_EAST,
  CA: RegionArea.NORTH_AMERICA_EAST,
  // na-west
  MX: RegionArea.NORTH_AMERICA_WEST,
  // sa-east
  AR: RegionArea.SOUTH_AMERICA_EAST,
  BR: RegionArea.SOUTH_AMERICA_EAST,
  CL: RegionArea.SOUTH_AMERICA_EAST,
  CO: RegionArea.SOUTH_AMERICA_EAST,
  PE: RegionArea.SOUTH_AMERICA_EAST,
  // me-central
  AE: RegionArea.MIDDLE_EAST_CENTRAL,
  BH: RegionArea.MIDDLE_EAST_CENTRAL,
  IL: RegionArea.MIDDLE_EAST_CENTRAL,
  KW: RegionArea.MIDDLE_EAST_CENTRAL,
  OM: RegionArea.MIDDLE_EAST_CENTRAL,
  QA: RegionArea.MIDDLE_EAST_CENTRAL,
  SA: RegionArea.MIDDLE_EAST_CENTRAL,
  // me-west
  EG: RegionArea.MIDDLE_EAST_WEST,
  JO: RegionArea.MIDDLE_EAST_WEST,
  LB: RegionArea.MIDDLE_EAST_WEST,
  TR: RegionArea.MIDDLE_EAST_WEST,
  // af-south
  ZA: RegionArea.AFRICA_SOUTH,
  NG: RegionArea.AFRICA_SOUTH,
  KE: RegionArea.AFRICA_SOUTH,
  // as-west
  RU: RegionArea.ASIA_WEST,
  KZ: RegionArea.ASIA_WEST,
  // as-south
  IN: RegionArea.ASIA_SOUTH,
  PK: RegionArea.ASIA_SOUTH,
  BD: RegionArea.ASIA_SOUTH,
  LK: RegionArea.ASIA_SOUTH,
  // as-east
  CN: RegionArea.ASIA_EAST,
  HK: RegionArea.ASIA_EAST,
  JP: RegionArea.ASIA_EAST,
  KR: RegionArea.ASIA_EAST,
  MY: RegionArea.ASIA_EAST,
  PH: RegionArea.ASIA_EAST,
  SG: RegionArea.ASIA_EAST,
  TH: RegionArea.ASIA_EAST,
  TW: RegionArea.ASIA_EAST,
  VN: RegionArea.ASIA_EAST,
  // au-south
  AU: RegionArea.AUSTRALIA_SOUTH,
  NZ: RegionArea.AUSTRALIA_SOUTH,
};

export const DEFAULT_REGION_BY_AREA: Partial<Record<RegionContinent, Region>> = {
  [RegionContinent.EUROPE]: Region.FRANKFURT,
  [RegionContinent.NORTH_AMERICA]: Region.VIRGINIA,
  [RegionContinent.SOUTH_AMERICA]: Region.SAO_PAULO,
  [RegionContinent.AFRICA]: Region.CAPE_TOWN,
  [RegionContinent.ASIA]: Region.MUMBAI,
  [RegionContinent.AUSTRALIA]: Region.SYDNEY,
};

export const DEFAULT_REGION_BY_CONTINENT: Partial<Record<RegionContinent, Region>> = {
  [RegionContinent.EUROPE]: Region.FRANKFURT,
  [RegionContinent.NORTH_AMERICA]: Region.VIRGINIA,
  [RegionContinent.SOUTH_AMERICA]: Region.SAO_PAULO,
  [RegionContinent.AFRICA]: Region.CAPE_TOWN,
  [RegionContinent.ASIA]: Region.SINGAPORE,
  [RegionContinent.AUSTRALIA]: Region.SYDNEY,
};

export const REGION_CONTINENT_SLUGS: Partial<Record<RegionContinent, string>> = {
  [RegionContinent.EUROPE]: "eu",
  [RegionContinent.NORTH_AMERICA]: "na",
  [RegionContinent.SOUTH_AMERICA]: "sa",
  [RegionContinent.MIDDLE_EAST]: "me",
  [RegionContinent.AFRICA]: "af",
  [RegionContinent.ASIA]: "as",
  [RegionContinent.AUSTRALIA]: "au",
};

export const REGION_SLUGS: Partial<Record<Region, string>> = {
  [Region.ZURICH]: "eu-zurich",
  [Region.FRANKFURT]: "eu-frankfurt",
  [Region.VIRGINIA]: "na-virginia",
  [Region.SAO_PAULO]: "sa-sao-paulo",
  [Region.CAPE_TOWN]: "af-cape-town",
  [Region.MUMBAI]: "as-mumbai",
  [Region.SINGAPORE]: "as-singapore",
  [Region.SYDNEY]: "au-sydney",
};

export function getRegionSlug(region: Region): string {
  return REGION_SLUGS[region] ?? Region[region].toLowerCase().replace("_", "-");
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
