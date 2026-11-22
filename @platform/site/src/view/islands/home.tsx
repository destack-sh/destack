import { PlainDate } from "@destack/schema";
import { Home } from "../page/home";

/** Render the home page, whose figures play throughout, its story told from a day. */
export default function HomeIsland(properties: { readonly today: string; readonly zone: string }) {
    return <Home today={PlainDate.parse(properties.today)} zone={properties.zone} />;
}
