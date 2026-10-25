import { isServer } from "@solidjs/web";
import * as solid from "@solidjs/router";
import type { Navigation } from "@destack/environment/browser";
import { useEnvironment } from "../page/view.ts";
import { navigationHistory } from "./history.ts";

export {
    createBeforeLeave,
    defineRoute,
    defineRoutes,
    int,
    RouterContext,
    useBeforeLeave,
    useHref,
    useIsRouting,
    useLinkState,
    useLocation,
    useMatch,
    useNavigate,
    useParams,
    usePreloadRoute,
    useResolvedPath,
    useRouteMatches,
    useSearchParams,
} from "@solidjs/router";
export type {
    BeforeLeaveEventArgs,
    DefaultSearchTypes,
    DefinedRoute,
    LinkState,
    Location,
    LocationChange,
    MatchFilter,
    MatchFilters,
    NavigateOptions,
    Navigator,
    OutputMatch,
    Params,
    PathEnd,
    PathMatch,
    PathParamsOf,
    RouteComponent,
    RouteDefinition,
    RouteDescription,
    RouteInfo,
    RouteMatch,
    RouteParams,
    RoutePaths,
    RoutePreloadFunc,
    RoutePreloadFuncArgs,
    RouteProps,
    RouterConfig,
    RouterInstance,
    RouterIntegration,
    RouterProps,
    RouterUtils,
    RouteSectionProps,
    SearchParams,
    SetParams,
    SetSearchParams,
    StandardSchemaV1,
    TypedMatchFilter,
    TypedPath,
    TypedSearchPath,
} from "@solidjs/router";

/** Create a router of a route tree over each tab's session history. */
export function createRouter<const Routes extends readonly solid.RouteDefinition[]>(
    configuration: solid.RouterConfig<Routes>,
): solid.RouterInstance<Routes> {
    // match and build paths without a tab, as the server renders
    const plain = solid.createRouter(configuration);
    const routers = new WeakMap<Navigation, solid.RouterInstance<Routes>>();

    // route each tab over its own navigation, unless the config brings its history
    const router = (properties: solid.RouterProps) => {
        if (isServer || configuration.history !== undefined) {
            return plain(properties);
        }
        const { navigation } = useEnvironment();
        const tab =
            routers.get(navigation) ??
            solid.createRouter({ ...configuration, history: navigationHistory(navigation) });
        routers.set(navigation, tab);

        return tab(properties);
    };

    // answer the plain router's paths, routes, config and matching
    return Object.assign(router, {
        paths: plain.paths,
        routes: plain.routes,
        config: plain.config,
        match: (url: string) => plain.match(url),
    });
}
