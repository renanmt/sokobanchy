//! Opt-in release captures of the real UI using Mesa's offscreen EGL renderer.
//! No desktop, saved progress, or reference images are read or changed.
use super::*;
use eframe::glow::{self, HasContext};
use std::{
    ffi::{CString, c_char, c_void},
    ptr,
    sync::Arc,
};

#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetPlatformDisplay(
        platform: u32,
        native: *mut c_void,
        attrs: *const isize,
    ) -> *mut c_void;
    fn eglInitialize(display: *mut c_void, major: *mut i32, minor: *mut i32) -> u32;
    fn eglBindAPI(api: u32) -> u32;
    fn eglChooseConfig(
        display: *mut c_void,
        attrs: *const i32,
        configs: *mut *mut c_void,
        size: i32,
        count: *mut i32,
    ) -> u32;
    fn eglCreatePbufferSurface(
        display: *mut c_void,
        config: *mut c_void,
        attrs: *const i32,
    ) -> *mut c_void;
    fn eglCreateContext(
        display: *mut c_void,
        config: *mut c_void,
        share: *mut c_void,
        attrs: *const i32,
    ) -> *mut c_void;
    fn eglMakeCurrent(
        display: *mut c_void,
        draw: *mut c_void,
        read: *mut c_void,
        context: *mut c_void,
    ) -> u32;
    fn eglGetProcAddress(name: *const c_char) -> *const c_void;
    fn eglDestroyContext(display: *mut c_void, context: *mut c_void) -> u32;
    fn eglDestroySurface(display: *mut c_void, surface: *mut c_void) -> u32;
    fn eglTerminate(display: *mut c_void) -> u32;
}

#[test]
#[ignore = "release capture: needs Mesa EGL; writes only the requested screenshot directory"]
fn capture_release_screenshots() {
    let output_dir = PathBuf::from(
        std::env::var_os("SOKOBANCHY_SCREENSHOT_DIR").expect("set SOKOBANCHY_SCREENSHOT_DIR"),
    );
    std::fs::create_dir_all(&output_dir).unwrap();
    let (width, height) = (1100_u32, 760_u32);
    // SAFETY: all EGL objects are checked, used on this thread, and destroyed
    // after the GL painter. Pointers refer to live arrays for each FFI call.
    unsafe {
        let display = eglGetPlatformDisplay(0x31dd, ptr::null_mut(), ptr::null()); // SURFACELESS_MESA
        assert!(!display.is_null());
        assert_ne!(eglInitialize(display, ptr::null_mut(), ptr::null_mut()), 0);
        assert_ne!(eglBindAPI(0x30a2), 0); // OPENGL_API
        let attrs = [
            0x3033, 1, 0x3040, 8, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3021, 8, 0x3038,
        ];
        let mut config = ptr::null_mut();
        let mut count = 0;
        assert_ne!(
            eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut count),
            0
        );
        assert_eq!(count, 1);
        let surface_attrs = [0x3057, width as i32, 0x3056, height as i32, 0x3038];
        let surface = eglCreatePbufferSurface(display, config, surface_attrs.as_ptr());
        assert!(!surface.is_null());
        let context = eglCreateContext(display, config, ptr::null_mut(), [0x3038].as_ptr());
        assert!(!context.is_null());
        assert_ne!(eglMakeCurrent(display, surface, surface, context), 0);
        let gl = Arc::new(glow::Context::from_loader_function(|name| {
            eglGetProcAddress(CString::new(name).unwrap().as_ptr())
        }));
        eprintln!(
            "Screenshot renderer: {}",
            gl.get_parameter_string(glow::RENDERER)
        );
        let mut painter = eframe::egui_glow::Painter::new(gl.clone(), "", None, false).unwrap();
        let ctx = egui::Context::default();
        let mut app = App::new(
            &ctx,
            Options {
                levels: None,
                theme: None,
                fresh: true,
            },
            parse_collection(include_str!("../assets/levels.xsb")).unwrap(),
        );
        app.theme = Theme::default();
        app.theme.apply(&ctx);
        app.select(20);
        for (name, page) in [
            ("game.png", Page::Game),
            ("puzzles.png", Page::Puzzles),
            ("collections.png", Page::Collections),
            ("about.png", Page::About),
            ("reset-all.png", Page::ResetAll),
            ("restart.png", Page::Restart),
        ] {
            if page == Page::Restart {
                app.move_player(Direction::Down);
                app.open_restart();
            } else {
                app.page = page;
            }
            app.puzzle_cursor = app.selected;
            app.scroll_to_puzzle = true;
            for _ in 0..5 {
                app.theme_checked = Instant::now();
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width as f32, height as f32),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                );
                let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
                painter.clear([width, height], [0.0, 0.0, 0.0, 1.0]);
                painter.paint_and_update_textures(
                    [width, height],
                    output.pixels_per_point,
                    &primitives,
                    &output.textures_delta,
                );
            }
            let mut pixels = vec![0; (width * height * 4) as usize];
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            let stride = width as usize * 4;
            let upright: Vec<u8> = pixels
                .chunks_exact(stride)
                .rev()
                .flatten()
                .copied()
                .collect();
            let file = std::fs::File::create(output_dir.join(name)).unwrap();
            let mut encoder = png::Encoder::new(file, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&upright)
                .unwrap();
        }
        painter.destroy();
        eglMakeCurrent(display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        eglDestroyContext(display, context);
        eglDestroySurface(display, surface);
        eglTerminate(display);
    }
}
