type ResizeHandleProps = {
    label: string;
    onPointerDown: (event: PointerEvent) => void;
};

export function ResizeHandle(props: ResizeHandleProps) {
    return (
        <button
            aria-label={props.label}
            class="hidden h-full min-h-0 w-full min-w-0 cursor-col-resize appearance-none border-0 bg-neutral-950 p-0 hover:bg-destack-accent lg:block"
            onPointerDown={props.onPointerDown}
            type="button"
        />
    );
}

export function dragHorizontally(event: PointerEvent, onMove: (clientX: number) => void) {
    event.preventDefault();

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture?.(event.pointerId);

    const move = (moveEvent: PointerEvent) => {
        onMove(moveEvent.clientX);
    };
    const stop = () => {
        target.releasePointerCapture?.(event.pointerId);
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", stop);
        window.removeEventListener("pointercancel", stop);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
}
