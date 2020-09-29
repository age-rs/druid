// Copyright 2018 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A textbox widget.

use std::time::Duration;

use crate::widget::prelude::*;
use crate::{
    BoxConstraints, Cursor, Data, Env, FontDescriptor, HotKey, KbKey, KeyOrValue, Point, Selector,
    Size, SysMods, TimerToken,
};

use crate::theme;

use crate::text::{EditAction, EditableText, Editor, TextStorage};

//const BORDER_WIDTH: f64 = 1.;
//const TEXT_INSETS: Insets = Insets::new(4.0, 2.0, 0.0, 2.0);

// we send ourselves this when we want to reset blink, which must be done in event.
const RESET_BLINK: Selector = Selector::new("druid-builtin.reset-textbox-blink");
const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox2<T> {
    //placeholder: String,
    editor: Editor<T>,
    cursor_timer: TimerToken,
    cursor_on: bool,
}

impl TextBox2<()> {
    /// Perform an `EditAction`. The payload *must* be an `EditAction`.
    pub const PERFORM_EDIT: Selector<EditAction> =
        Selector::new("druid-builtin.textbox.perform-edit");
}

impl<T> TextBox2<T> {
    /// Create a new TextBox widget
    pub fn new() -> Self {
        Self {
            editor: Editor::new().with_multi_line(true),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
        }
    }

    fn reset_cursor_blink(&mut self, ctx: &mut EventCtx) {
        self.cursor_on = true;
        self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
    }
}

impl<T: TextStorage + EditableText> Widget<T> for TextBox2<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                ctx.set_active(true);

                if !mouse.focus {
                    self.reset_cursor_blink(ctx);
                    self.editor.click(mouse, data);
                }

                ctx.request_paint();
            }
            Event::MouseMove(mouse) => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    self.editor.drag(mouse, data);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.request_paint();
                    self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
                }
            }
            Event::Command(ref cmd) if ctx.is_focused() && cmd.is(crate::commands::COPY) => {
                self.editor.copy(data);
                ctx.set_handled();
            }
            Event::Command(ref cmd) if ctx.is_focused() && cmd.is(crate::commands::CUT) => {
                self.editor.cut(data);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(RESET_BLINK) => self.reset_cursor_blink(ctx),
            Event::Command(cmd) if cmd.is(TextBox2::PERFORM_EDIT) => {
                let edit = cmd.get_unchecked(TextBox2::PERFORM_EDIT);
                self.editor.do_edit(edit.to_owned(), data);
            }
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    self.editor.paste(string.to_owned(), data);
                }
            }
            Event::KeyDown(key_event) => {
                match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KbKey::Tab).matches(k_e) => ctx.focus_next(),
                    k_e if HotKey::new(SysMods::Shift, KbKey::Tab).matches(k_e) => ctx.focus_prev(),
                    _ => self.editor.key(key_event, data),
                };
                self.reset_cursor_blink(ctx);

                ctx.request_paint();
            }
            _ => (),
        }
        //}
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.register_for_focus();
                self.editor.set_text(data.to_owned());
                self.editor.rebuild_if_needed(ctx.text(), env);
            }
            // an open question: should we be able to schedule timers here?
            LifeCycle::FocusChanged(true) => ctx.submit_command(RESET_BLINK.to(ctx.widget_id())),
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.editor.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        let size = bc.max();
        self.editor.set_wrap_width(size.width);
        self.editor.rebuild_if_needed(ctx.text(), env);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);
        let is_focused = ctx.is_focused();

        let rect = ctx.size().to_rect();

        // Paint the background
        ctx.fill(rect, &background_color);

        // Render text, selection, and cursor inside a clip
        // draw selection rects:
        for rect in self.editor.selection_rects() {
            ctx.fill(rect, &selection_color);
        }
        self.editor.draw(ctx, Point::ORIGIN);

        // Paint the cursor if focused and there's no selection
        if is_focused && self.cursor_on {
            let line = self.editor.cursor_line();
            ctx.stroke(line, &cursor_color, 1.);
        }
    }
}

impl<T> Default for TextBox2<T> {
    fn default() -> Self {
        TextBox2::new()
    }
}
