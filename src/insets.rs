use android_activity::{AndroidApp, Rect as AndroidRect};
use jni::objects::{Global, JObject, JValue};
use jni::{JavaVM, jni_sig, jni_str};
use log::{debug, error, info, warn};
use tiny_skia::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsetKind {
    /// System status bar at the top (or bottom) of the display.
    StatusBar,
    /// System navigation bar (buttons or gesture bar).
    NavigationBar,
    /// Display cutout area (e.g. camera hole, notch).
    Cutout,
    /// Window caption bar (desktop/freeform windowing mode).
    CaptionBar,
    /// Curved edge waterfall insets.
    Waterfall,
    /// Combined system bars fallback.
    SystemBars,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inset {
    pub kind: InsetKind,
    pub rect: Rect,
}

impl std::ops::Deref for Inset {
    type Target = Rect;
    fn deref(&self) -> &Self::Target {
        &self.rect
    }
}

/// Returns a `Vec<Inset>` containing each inset rectangle alongside its `InsetKind`.
pub fn get_insets(app: &AndroidApp) -> Vec<Inset> {
    info!("get insets!!!!!!!!");
    let insets = Vec::new();

    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        warn!("JavaVM or Activity pointer is null");
        return insets;
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) };
    let raw_activity = activity_ptr as jni::sys::jobject;

    let res = vm.attach_current_thread(|env| -> jni::errors::Result<Vec<Inset>> {
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
        let mut list = Vec::new();

        // 1. Get SDK level
        let sdk_int = env
            .get_static_field(
                jni_str!("android/os/Build$VERSION"),
                jni_str!("SDK_INT"),
                jni_sig!("I"),
            )
            .and_then(|v| v.i())
            .unwrap_or(0);

        // 2. Determine display / window dimensions
        let mut win_width: f32 = 0.0;
        let mut win_height: f32 = 0.0;

        if let Some(native_win) = app.native_window() {
            win_width = native_win.width() as f32;
            win_height = native_win.height() as f32;
        }

        let window = env
            .call_method(
                &activity,
                jni_str!("getWindow"),
                jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        let decor_view = if !window.as_raw().is_null() {
            env.call_method(
                &window,
                jni_str!("getDecorView"),
                jni_sig!("()Landroid/view/View;"),
                &[],
            )?
            .l()?
        } else {
            JObject::null()
        };

        // Fallback for window dimensions if NativeWindow is not available
        if win_width <= 0.0 || win_height <= 0.0 {
            if !decor_view.as_raw().is_null() {
                let w = env
                    .call_method(&decor_view, jni_str!("getWidth"), jni_sig!("()I"), &[])?
                    .i()?;
                let h = env
                    .call_method(&decor_view, jni_str!("getHeight"), jni_sig!("()I"), &[])?
                    .i()?;
                if w > 0 && h > 0 {
                    win_width = w as f32;
                    win_height = h as f32;
                }
            }
        }

        // On API 30+, we can also query WindowMetrics for bounds and insets
        let mut window_metrics = JObject::null();
        if sdk_int >= 30 {
            let wm = env
                .call_method(
                    &activity,
                    jni_str!("getWindowManager"),
                    jni_sig!("()Landroid/view/WindowManager;"),
                    &[],
                )?
                .l()?;
            if !wm.as_raw().is_null() {
                window_metrics = env
                    .call_method(
                        &wm,
                        jni_str!("getCurrentWindowMetrics"),
                        jni_sig!("()Landroid/view/WindowMetrics;"),
                        &[],
                    )?
                    .l()?;
                if (win_width <= 0.0 || win_height <= 0.0) && !window_metrics.as_raw().is_null() {
                    let bounds = env
                        .call_method(
                            &window_metrics,
                            jni_str!("getBounds"),
                            jni_sig!("()Landroid/graphics/Rect;"),
                            &[],
                        )?
                        .l()?;
                    if !bounds.as_raw().is_null() {
                        let l = env
                            .get_field(&bounds, jni_str!("left"), jni_sig!("I"))?
                            .i()? as f32;
                        let r = env
                            .get_field(&bounds, jni_str!("right"), jni_sig!("I"))?
                            .i()? as f32;
                        let t = env
                            .get_field(&bounds, jni_str!("top"), jni_sig!("I"))?
                            .i()? as f32;
                        let b = env
                            .get_field(&bounds, jni_str!("bottom"), jni_sig!("I"))?
                            .i()? as f32;
                        if r > l && b > t {
                            win_width = r - l;
                            win_height = b - t;
                        }
                    }
                }
            }
        }

        // 3. Obtain WindowInsets
        let mut insets_obj = JObject::null();
        if !decor_view.as_raw().is_null() {
            insets_obj = env
                .call_method(
                    &decor_view,
                    jni_str!("getRootWindowInsets"),
                    jni_sig!("()Landroid/view/WindowInsets;"),
                    &[],
                )?
                .l()?;
        }
        if insets_obj.as_raw().is_null() && sdk_int >= 30 && !window_metrics.as_raw().is_null() {
            insets_obj = env
                .call_method(
                    &window_metrics,
                    jni_str!("getWindowInsets"),
                    jni_sig!("()Landroid/view/WindowInsets;"),
                    &[],
                )?
                .l()?;
        }

        let mut got_cutout = false;
        let mut got_system_bars = false;

        if !insets_obj.as_raw().is_null() {
            // A. DisplayCutout (Camera holes, notches) - available since API 28
            if sdk_int >= 28 {
                let cutout = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getDisplayCutout"),
                        jni_sig!("()Landroid/view/DisplayCutout;"),
                        &[],
                    )?
                    .l()?;
                if !cutout.as_raw().is_null() {
                    let rect_list = env
                        .call_method(
                            &cutout,
                            jni_str!("getBoundingRects"),
                            jni_sig!("()Ljava/util/List;"),
                            &[],
                        )?
                        .l()?;
                    if !rect_list.as_raw().is_null() {
                        let size = env
                            .call_method(&rect_list, jni_str!("size"), jni_sig!("()I"), &[])?
                            .i()?;
                        for i in 0..size {
                            let rect_obj = env
                                .call_method(
                                    &rect_list,
                                    jni_str!("get"),
                                    jni_sig!("(I)Ljava/lang/Object;"),
                                    &[JValue::Int(i)],
                                )?
                                .l()?;
                            if !rect_obj.as_raw().is_null() {
                                let l = env
                                    .get_field(&rect_obj, jni_str!("left"), jni_sig!("I"))?
                                    .i()? as f32;
                                let t = env
                                    .get_field(&rect_obj, jni_str!("top"), jni_sig!("I"))?
                                    .i()? as f32;
                                let r = env
                                    .get_field(&rect_obj, jni_str!("right"), jni_sig!("I"))?
                                    .i()? as f32;
                                let b = env
                                    .get_field(&rect_obj, jni_str!("bottom"), jni_sig!("I"))?
                                    .i()? as f32;
                                if r > l && b > t {
                                    if let Some(rect) = Rect::from_ltrb(l, t, r, b) {
                                        list.push(Inset {
                                            kind: InsetKind::Cutout,
                                            rect,
                                        });
                                        got_cutout = true;
                                    }
                                }
                            }
                        }
                    }

                    // Waterfall insets (curved edge displays) - API 30+
                    if sdk_int >= 30 {
                        let waterfall = env
                            .call_method(
                                &cutout,
                                jni_str!("getWaterfallInsets"),
                                jni_sig!("()Landroid/graphics/Insets;"),
                                &[],
                            )?
                            .l()?;
                        if !waterfall.as_raw().is_null() {
                            let wl = env
                                .get_field(&waterfall, jni_str!("left"), jni_sig!("I"))?
                                .i()? as f32;
                            let wt = env
                                .get_field(&waterfall, jni_str!("top"), jni_sig!("I"))?
                                .i()? as f32;
                            let wr = env
                                .get_field(&waterfall, jni_str!("right"), jni_sig!("I"))?
                                .i()? as f32;
                            let wb = env
                                .get_field(&waterfall, jni_str!("bottom"), jni_sig!("I"))?
                                .i()? as f32;

                            if wl > 0.0 && win_height > 0.0 {
                                if let Some(rect) = Rect::from_ltrb(0.0, 0.0, wl, win_height) {
                                    list.push(Inset {
                                        kind: InsetKind::Waterfall,
                                        rect,
                                    });
                                }
                            }
                            if wr > 0.0 && win_width > 0.0 && win_height > 0.0 {
                                if let Some(rect) =
                                    Rect::from_ltrb(win_width - wr, 0.0, win_width, win_height)
                                {
                                    list.push(Inset {
                                        kind: InsetKind::Waterfall,
                                        rect,
                                    });
                                }
                            }
                            if wt > 0.0 && win_width > 0.0 {
                                if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, wt) {
                                    list.push(Inset {
                                        kind: InsetKind::Waterfall,
                                        rect,
                                    });
                                }
                            }
                            if wb > 0.0 && win_width > 0.0 && win_height > 0.0 {
                                if let Some(rect) =
                                    Rect::from_ltrb(0.0, win_height - wb, win_width, win_height)
                                {
                                    list.push(Inset {
                                        kind: InsetKind::Waterfall,
                                        rect,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // B. Navigation Bars & Status Bars
            if sdk_int >= 30 && win_width > 0.0 && win_height > 0.0 {
                // WindowInsets.Type:
                // statusBars() = 1 (1 << 0)
                // navigationBars() = 2 (1 << 1)
                // captionBar() = 4 (1 << 2)
                const TYPE_STATUS_BARS: i32 = 1;
                const TYPE_NAVIGATION_BARS: i32 = 2;
                const TYPE_CAPTION_BAR: i32 = 4;

                // 1. Navigation bar
                let nav_insets = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getInsets"),
                        jni_sig!("(I)Landroid/graphics/Insets;"),
                        &[JValue::Int(TYPE_NAVIGATION_BARS)],
                    )?
                    .l()?;
                if !nav_insets.as_raw().is_null() {
                    let nl = env
                        .get_field(&nav_insets, jni_str!("left"), jni_sig!("I"))?
                        .i()? as f32;
                    let nt = env
                        .get_field(&nav_insets, jni_str!("top"), jni_sig!("I"))?
                        .i()? as f32;
                    let nr = env
                        .get_field(&nav_insets, jni_str!("right"), jni_sig!("I"))?
                        .i()? as f32;
                    let nb = env
                        .get_field(&nav_insets, jni_str!("bottom"), jni_sig!("I"))?
                        .i()? as f32;

                    if nb > 0.0 {
                        if let Some(rect) =
                            Rect::from_ltrb(0.0, win_height - nb, win_width, win_height)
                        {
                            list.push(Inset {
                                kind: InsetKind::NavigationBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                    if nr > 0.0 {
                        if let Some(rect) =
                            Rect::from_ltrb(win_width - nr, 0.0, win_width, win_height)
                        {
                            list.push(Inset {
                                kind: InsetKind::NavigationBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                    if nl > 0.0 {
                        if let Some(rect) = Rect::from_ltrb(0.0, 0.0, nl, win_height) {
                            list.push(Inset {
                                kind: InsetKind::NavigationBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                    if nt > 0.0 {
                        if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, nt) {
                            list.push(Inset {
                                kind: InsetKind::NavigationBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                }

                // 2. Status bar
                let status_insets = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getInsets"),
                        jni_sig!("(I)Landroid/graphics/Insets;"),
                        &[JValue::Int(TYPE_STATUS_BARS)],
                    )?
                    .l()?;
                if !status_insets.as_raw().is_null() {
                    let st = env
                        .get_field(&status_insets, jni_str!("top"), jni_sig!("I"))?
                        .i()? as f32;
                    let sb = env
                        .get_field(&status_insets, jni_str!("bottom"), jni_sig!("I"))?
                        .i()? as f32;

                    if st > 0.0 {
                        if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, st) {
                            list.push(Inset {
                                kind: InsetKind::StatusBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                    if sb > 0.0 {
                        if let Some(rect) =
                            Rect::from_ltrb(0.0, win_height - sb, win_width, win_height)
                        {
                            list.push(Inset {
                                kind: InsetKind::StatusBar,
                                rect,
                            });
                            got_system_bars = true;
                        }
                    }
                }

                // 3. Caption bar
                let caption_insets = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getInsets"),
                        jni_sig!("(I)Landroid/graphics/Insets;"),
                        &[JValue::Int(TYPE_CAPTION_BAR)],
                    )?
                    .l()?;
                if !caption_insets.as_raw().is_null() {
                    let ct = env
                        .get_field(&caption_insets, jni_str!("top"), jni_sig!("I"))?
                        .i()? as f32;
                    if ct > 0.0 {
                        if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, ct) {
                            list.push(Inset {
                                kind: InsetKind::CaptionBar,
                                rect,
                            });
                        }
                    }
                }
            } else if win_width > 0.0 && win_height > 0.0 {
                // Pre-API 30: use getSystemWindowInset*()
                let sl = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getSystemWindowInsetLeft"),
                        jni_sig!("()I"),
                        &[],
                    )?
                    .i()? as f32;
                let st = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getSystemWindowInsetTop"),
                        jni_sig!("()I"),
                        &[],
                    )?
                    .i()? as f32;
                let sr = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getSystemWindowInsetRight"),
                        jni_sig!("()I"),
                        &[],
                    )?
                    .i()? as f32;
                let sb = env
                    .call_method(
                        &insets_obj,
                        jni_str!("getSystemWindowInsetBottom"),
                        jni_sig!("()I"),
                        &[],
                    )?
                    .i()? as f32;

                if st > 0.0 {
                    if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, st) {
                        list.push(Inset {
                            kind: InsetKind::StatusBar,
                            rect,
                        });
                        got_system_bars = true;
                    }
                }
                if sb > 0.0 {
                    if let Some(rect) = Rect::from_ltrb(0.0, win_height - sb, win_width, win_height)
                    {
                        list.push(Inset {
                            kind: InsetKind::NavigationBar,
                            rect,
                        });
                        got_system_bars = true;
                    }
                }
                if sr > 0.0 {
                    if let Some(rect) = Rect::from_ltrb(win_width - sr, 0.0, win_width, win_height)
                    {
                        list.push(Inset {
                            kind: InsetKind::NavigationBar,
                            rect,
                        });
                        got_system_bars = true;
                    }
                }
                if sl > 0.0 {
                    if let Some(rect) = Rect::from_ltrb(0.0, 0.0, sl, win_height) {
                        list.push(Inset {
                            kind: InsetKind::NavigationBar,
                            rect,
                        });
                        got_system_bars = true;
                    }
                }
            }
        }

        // 4. Fallback using system Resources if system bars were not yet retrieved (e.g. view not attached yet)
        if !got_system_bars && win_width > 0.0 && win_height > 0.0 {
            let resources = env
                .call_method(
                    &activity,
                    jni_str!("getResources"),
                    jni_sig!("()Landroid/content/res/Resources;"),
                    &[],
                )?
                .l()?;
            if !resources.as_raw().is_null() {
                let dimen_str = env.new_string("dimen")?;
                let android_pkg = env.new_string("android")?;

                // Status bar height
                let status_bar_name = env.new_string("status_bar_height")?;
                let status_id = env
                    .call_method(
                        &resources,
                        jni_str!("getIdentifier"),
                        jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"),
                        &[
                            JValue::Object(&status_bar_name),
                            JValue::Object(&dimen_str),
                            JValue::Object(&android_pkg),
                        ],
                    )?
                    .i()?;

                if status_id > 0 {
                    let h = env
                        .call_method(
                            &resources,
                            jni_str!("getDimensionPixelSize"),
                            jni_sig!("(I)I"),
                            &[JValue::Int(status_id)],
                        )?
                        .i()? as f32;
                    if h > 0.0 {
                        if let Some(rect) = Rect::from_ltrb(0.0, 0.0, win_width, h) {
                            list.push(Inset {
                                kind: InsetKind::StatusBar,
                                rect,
                            });
                        }
                    }
                }

                // Navigation bar height
                let nav_bar_name = env.new_string("navigation_bar_height")?;
                let nav_id = env
                    .call_method(
                        &resources,
                        jni_str!("getIdentifier"),
                        jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"),
                        &[
                            JValue::Object(&nav_bar_name),
                            JValue::Object(&dimen_str),
                            JValue::Object(&android_pkg),
                        ],
                    )?
                    .i()?;

                if nav_id > 0 {
                    let h = env
                        .call_method(
                            &resources,
                            jni_str!("getDimensionPixelSize"),
                            jni_sig!("(I)I"),
                            &[JValue::Int(nav_id)],
                        )?
                        .i()? as f32;
                    if h > 0.0 {
                        if let Some(rect) =
                            Rect::from_ltrb(0.0, win_height - h, win_width, win_height)
                        {
                            list.push(Inset {
                                kind: InsetKind::NavigationBar,
                                rect,
                            });
                        }
                    }
                }
            }
        }

        Ok(list)
    });

    match res {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to retrieve insets via JNI: {e:?}");
            Vec::new()
        }
    }
}
