use super::common::{
    assert_media_event, decode_media_event, decode_string, media_watch_options_harness_value,
    path_harness_value, string_array_harness_value, with_desktop_media_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::MediaWatchOptionsValue;
use crate::tests::platform::assert_runtime_error_code;

/// Exercise desktop media watch add or update or remove events through both harnesses.
#[test]
fn test_media_desktop_watch_reports_add_update_and_remove_events() {
    with_desktop_media_context("watch-events", |mut context, roots| {
        let imported_path = roots.base_directory.join("watch-import.jpg");

        // open one watch that tracks all event kinds
        let options = media_watch_options_harness_value(
            &mut context,
            MediaWatchOptionsValue {
                kinds: Vec::new(),
                include_hidden: false,
                include_added: true,
                include_updated: true,
                include_removed: true,
            },
        );
        let handle = context.destack_os_media_watch_open(options)?;

        // unchanged state should not emit one event
        let error = match context.destack_os_media_watch_try_read(handle) {
            Ok(_) => panic!("mediaWatchTryRead should report ioWouldBlock without one change"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoWouldBlock);

        // import one new asset and observe one added event
        std::fs::write(&imported_path, b"watch-add")
            .expect("desktop media watch import should write one file");
        let imported_path = path_harness_value(&mut context, &imported_path)?;
        let imported_id =
            context.destack_os_media_import_path(imported_path, MediaAssetKind::Image)?;
        let imported_id = decode_string(&mut context, imported_id)?;
        let event = context.destack_os_media_watch_read(handle, 50_000_000)?;
        let event = decode_media_event(&mut context, event)?;

        assert_media_event(&event, "added", &imported_id);

        // mutate the imported asset and observe one updated event
        let imported_file = roots.pictures_directory.join("watch-import.jpg");
        std::fs::write(&imported_file, b"watch-update-payload")
            .expect("desktop media watch update should rewrite one file");
        let event = context.destack_os_media_watch_read(handle, 50_000_000)?;
        let event = decode_media_event(&mut context, event)?;

        assert_media_event(&event, "updated", &imported_id);

        // delete the imported asset and observe one removed event
        let ids = string_array_harness_value(&mut context, &[imported_id.as_str()])?;
        context.destack_os_media_delete(ids)?;
        let event = context.destack_os_media_watch_read(handle, 50_000_000)?;
        let event = decode_media_event(&mut context, event)?;

        assert_media_event(&event, "removed", &imported_id);

        context.destack_os_media_watch_close(handle)?;

        Ok(())
    })
    .expect("desktop media watch event stream should succeed")
}

/// Verify closing one media watch invalidates the handle.
#[test]
fn test_media_desktop_watch_close_invalidates_the_handle() {
    with_desktop_media_context("watch-close", |mut context, _roots| {
        // close one watch then verify later reads reject it
        let options = media_watch_options_harness_value(
            &mut context,
            MediaWatchOptionsValue {
                kinds: Vec::new(),
                include_hidden: false,
                include_added: true,
                include_updated: true,
                include_removed: true,
            },
        );
        let handle = context.destack_os_media_watch_open(options)?;
        context.destack_os_media_watch_close(handle)?;

        let error = match context.destack_os_media_watch_try_read(handle) {
            Ok(_) => panic!("mediaWatchTryRead should reject one closed handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    })
    .expect("desktop media watch close should invalidate the handle")
}
