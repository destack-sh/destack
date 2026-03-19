use super::common::{
    decode_media_page, decode_string, media_query_harness_value, path_harness_value,
    string_array_harness_value, with_desktop_media_context,
};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::MediaQueryValue;

/// Exercise desktop media import and delete through both harnesses.
#[test]
fn test_media_desktop_import_and_delete_roundtrip() {
    with_desktop_media_context("import-delete", |mut context, roots| {
        let import_path = roots.base_directory.join("import.jpg");
        std::fs::write(&import_path, b"import").expect("desktop media import file should write");

        // import one new asset into the desktop pictures root
        let import_path = path_harness_value(&mut context, &import_path)?;
        let imported_id =
            context.destack_os_media_import_path(import_path, MediaAssetKind::Image)?;
        let imported_id = decode_string(&mut context, imported_id)?;

        // verify the imported asset is visible through image queries
        let query = media_query_harness_value(
            &mut context,
            MediaQueryValue {
                cursor: None,
                limit: Some(16),
                kinds: vec![MediaAssetKind::Image],
                include_hidden: false,
            },
        );
        let page = context.destack_os_media_list(query)?;
        let page = decode_media_page(&mut context, page)?;

        assert!(page.assets.iter().any(|asset| asset.id == imported_id));

        // then delete the imported asset and ensure it disappears
        let ids = string_array_harness_value(&mut context, &[imported_id.as_str()])?;
        context.destack_os_media_delete(ids)?;

        let query = media_query_harness_value(
            &mut context,
            MediaQueryValue {
                cursor: None,
                limit: Some(16),
                kinds: vec![MediaAssetKind::Image],
                include_hidden: false,
            },
        );
        let page = context.destack_os_media_list(query)?;
        let page = decode_media_page(&mut context, page)?;

        assert!(!page.assets.iter().any(|asset| asset.id == imported_id));

        Ok(())
    })
    .expect("desktop media import and delete should succeed")
}
