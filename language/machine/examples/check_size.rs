use destack_machine::{
    Frame, HeapHandle, RawPointer, StackPointer, ThreadedBlock, ThreadedFunction,
    ThreadedInstruction, ThreadedState, Value,
};

fn main() {
    println!("Frame: {} bytes", std::mem::size_of::<Frame>());
    println!(
        "ThreadedBlock: {} bytes",
        std::mem::size_of::<ThreadedBlock>()
    );
    println!(
        "ThreadedFunction: {} bytes",
        std::mem::size_of::<ThreadedFunction>()
    );
    println!(
        "ThreadedInstruction: {} bytes",
        std::mem::size_of::<ThreadedInstruction>()
    );
    println!(
        "ThreadedState: {} bytes",
        std::mem::size_of::<ThreadedState<'_>>()
    );

    println!("Value: {} bytes", std::mem::size_of::<Value>());
    println!("HeapHandle: {} bytes", std::mem::size_of::<HeapHandle>());
    println!("RawPointer: {} bytes", std::mem::size_of::<RawPointer>());
    println!(
        "StackPointer: {} bytes",
        std::mem::size_of::<StackPointer>()
    );
    println!(
        "Option<Value>: {} bytes",
        std::mem::size_of::<Option<Value>>()
    );
}
