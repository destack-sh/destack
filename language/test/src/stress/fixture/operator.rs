use std::fmt::Write;

/// Generate dense operator precedence surfaces.
pub(super) fn operator_forms(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 360);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const arithmetic{index} = ((a{index} + b{index}) * c{index} - d{index} / e{index}) ** p{index};"
        );
        let _ = writeln!(
            source,
            "const logical{index} = a{index} && b{index} || c{index} ?? fallback{index};"
        );
        let _ = writeln!(
            source,
            "const shifts{index} = (a{index} << b{index}) | (c{index} >> d{index}) ^ (e{index} >>> f{index});"
        );

        let _ = writeln!(
            source,
            "const deref{index} = (*box{index}).value + (&readonly value{index}).field;"
        );
    }

    source
}
