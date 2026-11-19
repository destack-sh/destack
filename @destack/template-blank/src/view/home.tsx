import { t } from "@destack/locale";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import { useLocale } from "@destack/view";

/** Greet the person on the package's start page, the first view to edit. */
export default function Home() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`A blank package`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(t`Edit src/view/home.tsx, and this page shows it at once.`)}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}
