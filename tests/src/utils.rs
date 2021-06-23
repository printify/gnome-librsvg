#![cfg(test)]
#![allow(unused)]

use cairo;
use gio;
use glib;
use glib::translate::*;
use libc;
use std::env;
use std::ffi::CString;
use std::sync::Once;

use librsvg::{
    surface_utils::shared_surface::{SharedImageSurface, SurfaceType},
    CairoRenderer, Loader, LoadingError, RenderingError, SvgHandle,
};

pub fn load_svg(input: &'static [u8]) -> Result<SvgHandle, LoadingError> {
    let bytes = glib::Bytes::from_static(input);
    let stream = gio::MemoryInputStream::from_bytes(&bytes);

    Loader::new().read_stream(&stream, None::<&gio::File>, None::<&gio::Cancellable>)
}

#[derive(Copy, Clone)]
pub struct SurfaceSize(pub i32, pub i32);

pub fn render_document<F: FnOnce(&cairo::Context)>(
    svg: &SvgHandle,
    surface_size: SurfaceSize,
    cr_transform: F,
    viewport: cairo::Rectangle,
) -> Result<SharedImageSurface, RenderingError> {
    let renderer = CairoRenderer::new(svg);

    let SurfaceSize(width, height) = surface_size;

    let output = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height).unwrap();

    let res = {
        let cr = cairo::Context::new(&output).expect("Failed to create a cairo context");
        cr_transform(&cr);
        Ok(renderer.render_document(&cr, &viewport)?)
    };

    res.and_then(|_| Ok(SharedImageSurface::wrap(output, SurfaceType::SRgb)?))
}

#[cfg(system_deps_have_pangoft2)]
mod pango_ft2 {
    use super::*;
    use fontconfig_sys::fontconfig;
    use glib::prelude::*;
    use pangocairo::FontMap;

    extern "C" {
        // pango_fc_font_map_set_config (PangoFcFontMap *fcfontmap,
        //                               FcConfig       *fcconfig);
        // This is not bound in gtk-rs, and PangoFcFontMap is not even exposed, so we'll bind it by hand.
        fn pango_fc_font_map_set_config(
            font_map: *mut libc::c_void,
            config: *mut fontconfig::FcConfig,
        );
    }

    pub unsafe fn load_test_fonts() {
        let font_paths = [
            "tests/resources/Roboto-Regular.ttf",
            "tests/resources/Roboto-Italic.ttf",
            "tests/resources/Roboto-Bold.ttf",
            "tests/resources/Roboto-BoldItalic.ttf",
        ];

        let config = fontconfig::FcConfigCreate();

        for path in &font_paths {
            let path_cstring = CString::new(*path).unwrap();

            if fontconfig::FcConfigAppFontAddFile(config, path_cstring.as_ptr() as *const _) == 0 {
                panic!("Could not load font file {} for tests; aborting", path,);
            }
        }

        let font_map = FontMap::for_font_type(cairo::FontType::FontTypeFt).unwrap();
        let raw_font_map: *mut pango::ffi::PangoFontMap = font_map.to_glib_none().0;

        pango_fc_font_map_set_config(raw_font_map as *mut _, config);
        fontconfig::FcConfigDestroy(config);

        FontMap::set_default(Some(font_map.downcast::<pangocairo::FontMap>().unwrap()));
    }
}

#[cfg(system_deps_have_pangoft2)]
pub fn setup_font_map() {
    unsafe {
        self::pango_ft2::load_test_fonts();
    }
}

#[cfg(not(system_deps_have_pangoft2))]
pub fn setup_font_map() {}

pub fn setup_language() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        // For systemLanguage attribute tests.
        // The trailing ":" is intentional to test gitlab#425.
        env::set_var("LANGUAGE", "de:en_US:en:");
        env::set_var("LC_ALL", "de:en_US:en:");
    });
}
