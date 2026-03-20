use super::common::{
    decode_media_descriptor, decode_media_page, media_query_harness_value, string_harness_value,
    with_desktop_media_context,
};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::MediaQueryValue;

/// Exercise desktop media list and read through both harnesses.
#[test]
fn test_media_desktop_list_and_read_roundtrip() {
    with_desktop_media_context("list-read", |mut context, _roots| {
        // list all visible assets through the desktop media bridge
        let query = media_query_harness_value(
            &mut context,
            MediaQueryValue {
                cursor: None,
                limit: Some(16),
                kinds: Vec::new(),
                include_hidden: false,
            },
        );
        let page = context.destack_os_media_list(query)?;
        let page = decode_media_page(&mut context, page)?;
        let mut file_names = page
            .assets
            .iter()
            .map(|asset| asset.filename.clone())
            .collect::<Vec<_>>();
        file_names.sort();

        assert_eq!(file_names, vec!["photo.png", "song.mp3", "video.mp4"]);

        // then read one listed asset back by its stable identifier
        let id = page
            .assets
            .iter()
            .find(|asset| asset.filename == "photo.png")
            .map(|asset| asset.id.clone())
            .expect("desktop media listing should include one photo asset");
        let read_id = string_harness_value(&mut context, &id);
        let descriptor = context.destack_os_media_read(read_id)?;
        let descriptor = decode_media_descriptor(&mut context, descriptor)?;

        assert_eq!(descriptor.filename, "photo.png");
        assert_eq!(descriptor.kind, MediaAssetKind::Image);
        assert_eq!(descriptor.mime_type, "");
        assert_eq!(descriptor.width, 0);
        assert_eq!(descriptor.height, 0);
        assert_eq!(descriptor.duration_ms, 0);
        assert_eq!(descriptor.size_bytes, 68);
        assert_eq!(descriptor.uri, id);

        Ok(())
    })
    .expect("desktop media list and read should succeed")
}
