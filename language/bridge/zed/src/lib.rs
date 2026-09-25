use zed_extension_api as zed;

mod extension;

use extension::TsppExtension;

zed::register_extension!(TsppExtension);
