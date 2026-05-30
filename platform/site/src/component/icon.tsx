type IconProps = {
    class?: string;
};

export function Icon(props: IconProps) {
    return (
        <img
            alt=""
            class={props.class}
            decoding="async"
            height="64"
            src="/brand/favicon/favicon-simple.svg"
            width="64"
        />
    );
}
