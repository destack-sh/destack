use crate::stress::StressFixture;
use crate::stress::file::write_complete_file;

/// Write one formatted stress output file.
pub(super) fn write_formatted_output(
    fixture: &StressFixture,
    profile: &str,
    output: &str,
) -> Result<(), String> {
    let Some(stem) = fixture.path.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(format!(
            "stress fixture has no utf-8 file stem: {}",
            fixture.path.display()
        ));
    };
    let Some(extension) = fixture
        .path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return Err(format!(
            "stress fixture has no utf-8 extension: {}",
            fixture.path.display()
        ));
    };

    let output_name = format!("{stem}.{profile}.formatted.{extension}");
    let output_path = fixture.path.with_file_name(output_name);

    write_complete_file(&output_path, output)
}
