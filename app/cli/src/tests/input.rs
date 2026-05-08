use std::path::PathBuf;

use crate::common::{InputArgs, InputSource};

/// Preserves input ordering and naming for sources.
#[test]
fn test_input_args_to_sources_orders_inputs() {
    // set up input args with all sources
    let args = InputArgs {
        files: vec![PathBuf::from("src/main.ds")],
        eval: vec!["let x = 1".to_string()],
        module: vec!["mod:export const value = 1".to_string()],
        stdin: true,
        file_type: Some("ts".to_string()),
    };

    // resolve the sources
    let sources = args.to_sources().expect("input sources should resolve");

    // assert source ordering
    assert_eq!(sources.len(), 4);
    assert!(matches!(sources[0], InputSource::File(_)));
    assert!(matches!(sources[1], InputSource::Inline { .. }));
    assert!(matches!(sources[2], InputSource::Inline { .. }));
    assert!(matches!(sources[3], InputSource::Stdin { .. }));

    // assert resolved names
    match &sources[1] {
        InputSource::Inline { name, .. } => {
            assert_eq!(name, "<eval0>.ts");
        }
        _ => unreachable!(),
    }
    match &sources[2] {
        InputSource::Inline { name, .. } => {
            assert_eq!(name, "mod.ts");
        }
        _ => unreachable!(),
    }
    match &sources[3] {
        InputSource::Stdin { name } => {
            assert_eq!(name, "<stdin>.ts");
        }
        _ => unreachable!(),
    }
}
