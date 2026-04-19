use core::default::Default;
use parley::{GenericFamily, StyleProperty, editing::SplitString};
use std::time::{Duration, Instant};
use vello_svg::vello::{
    kurbo::Point,
    peniko::{Brush, color::palette},
};
use winit::{event::Modifiers, keyboard::NamedKey};

pub(crate) use parley::editing::Generation;
use parley::{FontContext, LayoutContext, PlainEditor, PlainEditorDriver};

use crate::Key;

#[derive(Clone)]
pub(crate) struct Editor {
    pub(crate) editor: PlainEditor<Brush>,
    pub(crate) last_click_time: Option<Instant>,
    pub(crate) click_count: u32,
    pub(crate) pointer_down: bool,
    pub(crate) cursor_pos: (f32, f32),
    pub(crate) cursor_visible: bool,
    pub(crate) modifiers: Option<Modifiers>,
    pub(crate) start_time: Option<Instant>,
    pub(crate) blink_period: Duration,
}

#[allow(dead_code)]
impl Editor {
    pub fn new(text: &str) -> Self {
        let mut editor = PlainEditor::new(32.0);
        editor.set_text(text);
        editor.set_scale(1.0);
        let styles = editor.edit_styles();
        styles.insert(StyleProperty::LineHeight(
            parley::LineHeight::FontSizeRelative(1.2),
        ));
        styles.insert(GenericFamily::SystemUi.into());
        styles.insert(StyleProperty::Brush(palette::css::WHITE.into()));
        Self {
            editor,
            last_click_time: Default::default(),
            click_count: Default::default(),
            pointer_down: Default::default(),
            cursor_pos: Default::default(),
            cursor_visible: Default::default(),
            modifiers: Default::default(),
            start_time: Default::default(),
            blink_period: Default::default(),
        }
    }

    fn driver<'a>(
        &'a mut self,
        font_context: &'a mut FontContext,
        layout_context: &'a mut LayoutContext<Brush>,
    ) -> PlainEditorDriver<'a, Brush> {
        self.editor.driver(font_context, layout_context)
    }

    pub fn editor(&mut self) -> &mut PlainEditor<Brush> {
        &mut self.editor
    }

    pub fn text(&self) -> SplitString<'_> {
        self.editor.text()
    }

    pub fn cursor_reset(&mut self) {
        self.start_time = Some(Instant::now());
        // TODO: for real world use, this should be reading from the system settings
        self.blink_period = Duration::from_millis(500);
        self.cursor_visible = true;
    }

    pub fn disable_blink(&mut self) {
        self.start_time = None;
    }

    pub fn next_blink_time(&self) -> Option<Instant> {
        self.start_time.map(|start_time| {
            let phase = Instant::now().duration_since(start_time);

            start_time
                + Duration::from_nanos(
                    ((phase.as_nanos() / self.blink_period.as_nanos() + 1)
                        * self.blink_period.as_nanos()) as u64,
                )
        })
    }

    pub fn cursor_blink(&mut self) {
        self.cursor_visible = self.start_time.is_some_and(|start_time| {
            let elapsed = Instant::now().duration_since(start_time);
            (elapsed.as_millis() / self.blink_period.as_millis()).is_multiple_of(2)
        });
    }

    /// Return the current `Generation` of the layout.
    pub fn generation(&self) -> Generation {
        self.editor.generation()
    }

    pub fn handle_key(
        &mut self,
        key: Key,
        layout_cx: &mut LayoutContext<Brush>,
        font_cx: &mut FontContext,
        modifiers: Option<Modifiers>,
    ) {
        self.modifiers = modifiers;
        #[allow(unused)]
        let (shift, action_mod, alt) = self
            .modifiers
            .map(|mods| {
                (
                    mods.state().shift_key(),
                    if cfg!(target_os = "macos") {
                        mods.state().super_key()
                    } else {
                        mods.state().control_key()
                    },
                    mods.state().alt_key(),
                )
            })
            .unwrap_or_default();
        let mut drv = self.driver(font_cx, layout_cx);
        match key {
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            Key::Character(c) if action_mod && matches!(c.as_str(), "c" | "x" | "v") => {
                use clipboard_rs::{Clipboard, ClipboardContext};
                match c.to_lowercase().as_str() {
                    "c" => {
                        if let Some(text) = drv.editor.selected_text() {
                            let cb = ClipboardContext::new().unwrap();
                            cb.set_text(text.to_owned()).ok();
                        }
                    }
                    "x" => {
                        if let Some(text) = drv.editor.selected_text() {
                            let cb = ClipboardContext::new().unwrap();
                            cb.set_text(text.to_owned()).ok();
                            drv.delete_selection();
                        }
                    }
                    "v" => {
                        let cb = ClipboardContext::new().unwrap();
                        let text = cb.get_text().unwrap_or_default();
                        drv.insert_or_replace_selection(&text);
                    }
                    _ => (),
                }
            }
            Key::Character(c) if action_mod && matches!(c.to_lowercase().as_str(), "a") => {
                if shift {
                    drv.collapse_selection();
                } else {
                    drv.select_all();
                }
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if action_mod {
                    if shift {
                        drv.select_to_line_start();
                    } else {
                        drv.move_to_line_start();
                    }
                } else if alt {
                    if shift {
                        drv.select_word_left();
                    } else {
                        drv.move_word_left();
                    }
                } else if shift {
                    drv.select_left();
                } else {
                    drv.move_left();
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                if action_mod {
                    if shift {
                        drv.select_to_line_end();
                    } else {
                        drv.move_to_line_end();
                    }
                } else if alt {
                    if shift {
                        drv.select_word_right();
                    } else {
                        drv.move_word_right();
                    }
                } else if shift {
                    drv.select_right();
                } else {
                    drv.move_right();
                }
            }
            Key::Named(NamedKey::ArrowUp) => {
                if action_mod {
                    if shift {
                        drv.select_to_text_start();
                    } else {
                        drv.move_to_text_start();
                    }
                } else if shift {
                    drv.select_up();
                } else {
                    drv.move_up();
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if action_mod {
                    if shift {
                        drv.select_to_text_end();
                    } else {
                        drv.move_to_text_end();
                    }
                } else if shift {
                    drv.select_down();
                } else {
                    drv.move_down();
                }
            }
            Key::Named(NamedKey::Home) => {
                if action_mod {
                    if shift {
                        drv.select_to_text_start();
                    } else {
                        drv.move_to_text_start();
                    }
                } else if shift {
                    drv.select_to_line_start();
                } else {
                    drv.move_to_line_start();
                }
            }
            Key::Named(NamedKey::End) => {
                if action_mod {
                    if shift {
                        drv.select_to_text_end();
                    } else {
                        drv.move_to_text_end();
                    }
                } else if shift {
                    drv.select_to_line_end();
                } else {
                    drv.move_to_line_end();
                }
            }
            Key::Named(NamedKey::Delete) => {
                if action_mod {
                    drv.delete_word();
                } else {
                    drv.delete();
                }
            }
            Key::Named(NamedKey::Backspace) => {
                if action_mod {
                    drv.select_to_line_start();
                    drv.delete_selection();
                } else if alt {
                    drv.backdelete_word();
                } else {
                    drv.backdelete();
                }
            }
            Key::Named(NamedKey::Enter) => {
                drv.insert_or_replace_selection("\n");
            }
            Key::Named(NamedKey::Space) => {
                drv.insert_or_replace_selection(" ");
            }
            Key::Character(s) => {
                drv.insert_or_replace_selection(&s);
            }
            _ => (),
        }
    }
    pub(crate) fn mouse_pressed(
        &mut self,
        layout_cx: &mut LayoutContext<Brush>,
        font_cx: &mut FontContext,
    ) {
        self.cursor_reset();
        if !self.editor.is_composing() {
            self.pointer_down = true;
            let now = Instant::now();
            if let Some(last) = self.last_click_time.take() {
                if now.duration_since(last).as_secs_f64() < 0.25 {
                    self.click_count = (self.click_count + 1) % 4;
                } else {
                    self.click_count = 1;
                }
            } else {
                self.click_count = 1;
            }
            self.last_click_time = Some(now);
            let click_count = self.click_count;
            let cursor_pos = self.cursor_pos;
            let mut drv = self.editor.driver(font_cx, layout_cx);
            match click_count {
                2 => drv.select_word_at_point(cursor_pos.0, cursor_pos.1),
                3 => drv.select_line_at_point(cursor_pos.0, cursor_pos.1),
                _ => drv.move_to_point(cursor_pos.0, cursor_pos.1),
            }
        }
    }
    pub(crate) fn mouse_released(&mut self) {
        self.pointer_down = false;
    }
    pub(crate) fn mouse_moved(
        &mut self,
        position: Point,
        layout_cx: &mut LayoutContext<Brush>,
        font_cx: &mut FontContext,
    ) {
        let prev_pos = self.cursor_pos;
        self.cursor_pos = (position.x as f32, position.y as f32);
        // macOS seems to generate a spurious move after selecting word?
        if self.pointer_down && prev_pos != self.cursor_pos && !self.editor.is_composing() {
            self.cursor_reset();
            let cursor_pos = self.cursor_pos;
            self.driver(font_cx, layout_cx)
                .extend_selection_to_point(cursor_pos.0, cursor_pos.1);
        }
    }
}
