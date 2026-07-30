use zed_extension_api as zed;

mod extension;

use extension::DestackExtension;

zed::register_extension!(DestackExtension);
