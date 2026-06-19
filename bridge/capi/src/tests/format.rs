use std::ptr;

use crate::{
    DestackFormatOutput, DestackRevision, destack_format_output_destroy,
    destack_format_request_destroy, destack_revision_destroy, destack_session_format,
    destack_session_revision,
};

use super::session::{TestSession, assert_ok, format_text_request, string};

#[test]
fn test_format_source() {
    let session = TestSession::open(&[("destack.json", r#"{"name":"@test/app"}"#)]);
    let mut error = ptr::null_mut();
    let mut revision = DestackRevision::empty();
    let mut output = DestackFormatOutput {
        text: ptr::null_mut(),
    };
    let mut request = format_text_request("src/index.ds", "export  const value=1;");

    // read current revision
    let status = unsafe { destack_session_revision(session.inner, &mut revision, &mut error) };
    assert_ok(status, error);

    // copy revision value as C would for by value arguments
    let revision_argument = unsafe { ptr::read(&revision) };

    // format ad hoc source text
    let status = unsafe {
        destack_session_format(
            session.inner,
            revision_argument,
            &request,
            &mut output,
            &mut error,
        )
    };
    assert_ok(status, error);

    assert_eq!(string(output.text), "export const value = 1;\n");

    // release returned and input values
    unsafe {
        destack_format_output_destroy(&mut output);
        destack_format_request_destroy(&mut request);
        destack_revision_destroy(&mut revision);
    }
}
