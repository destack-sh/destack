use zed_extension_api as zed;

mod language;

use language::DestackExtension;

zed::register_extension!(DestackExtension);
