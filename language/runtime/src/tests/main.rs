use destack_runtime::tests::affinity::run_display_main_thread_case;

/// Run one named affinity-sensitive display test case on the process main thread.
fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut arguments = arguments.into_iter();
    let mut case_name = None;

    while let Some(argument) = arguments.next() {
        // parse one named argument pair
        if argument == "--case" {
            case_name = arguments.next();
            continue;
        }
    }

    // allow cargo test to execute this harness-free target without helper arguments
    let Some(case_name) = case_name else {
        return;
    };

    run_display_main_thread_case(case_name.as_str());
}
