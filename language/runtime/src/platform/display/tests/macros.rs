/// Return one stable libtest path for the current test function.
macro_rules! display_case_name {
    ($case:ident) => {
        concat!(module_path!(), "::", stringify!($case))
    };
}
