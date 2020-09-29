use super::{
    movement, offset_for_delete_backwards, BasicTextInput, EditAction, EditableText, MouseAction,
    Movement, Selection, TextInput, TextLayout, TextStorage,
};
use crate::kurbo::Line;
use crate::piet::PietText;
use crate::{Application, Env, KeyEvent, MouseEvent, PaintCtx, Point, Rect, UpdateCtx};

#[derive(Debug, Clone)]
pub struct Editor<T> {
    layout: TextLayout<T>,
    selection: Selection,
    multi_line: bool,
    fixed_width: f64,
    // this can be Box<dyn TextInput> in the future
    editor: BasicTextInput,
}

impl<T> Editor<T> {
    pub fn new() -> Self {
        Editor {
            layout: TextLayout::new(),
            selection: Selection::caret(0),
            multi_line: false,
            fixed_width: f64::INFINITY,
            editor: BasicTextInput::default(),
        }
    }

    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = multi_line;
        self
    }

    pub fn set_wrap_width(&mut self, width: f64) {
        self.layout.set_wrap_width(width);
    }
}

impl<T: TextStorage + EditableText> Editor<T> {
    pub fn set_text(&mut self, text: T) {
        self.layout.set_text(text)
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn selection_rects(&self) -> Vec<Rect> {
        self.layout.rects_for_range(self.selection.range())
    }

    pub fn cursor_line(&self) -> Line {
        self.layout
            .cursor_line_for_text_position(self.selection.end)
    }

    pub fn click(&mut self, mouse: &MouseEvent, data: &mut T) {
        self.do_edit(EditAction::Click(self.mouse_action_for_event(mouse)), data);
    }

    pub fn drag(&mut self, mouse: &MouseEvent, data: &mut T) {
        self.do_edit(EditAction::Drag(self.mouse_action_for_event(mouse)), data);
    }

    fn mouse_action_for_event(&self, event: &MouseEvent) -> MouseAction {
        let pos = self.layout.text_position_for_point(event.pos);
        MouseAction {
            row: 0,
            column: pos,
            mods: event.mods,
        }
    }

    pub fn key(&mut self, action: &KeyEvent, data: &mut T) {
        if let Some(edit) = self.editor.handle_event(action) {
            self.do_edit(edit, data)
        }
    }

    pub fn update(&mut self, ctx: &mut UpdateCtx, new_data: &T, env: &Env) {
        if self.data_is_stale(new_data) {
            self.layout.set_text(new_data.clone());
            self.selection.constrain_to(new_data);
            ctx.request_paint();
        } else if self.layout.needs_rebuild_after_update(ctx) {
            ctx.request_paint();
        }
        self.rebuild_if_needed(ctx.text(), env);
    }

    /// Must be called in WidgetAdded
    pub fn rebuild_if_needed(&mut self, factory: &mut PietText, env: &Env) {
        self.layout.rebuild_if_needed(factory, env);
    }

    pub fn do_edit(&mut self, edit: EditAction, data: &mut T) {
        if self.data_is_stale(data) {
            log::warn!("editor data changed externally, skipping event {:?}", &edit);
            return;
        }
        match edit {
            EditAction::Insert(chars) | EditAction::Paste(chars) => self.insert(&chars, data),
            EditAction::Backspace => self.delete_backward(data),
            EditAction::Delete => self.delete_forward(data),
            EditAction::JumpDelete(mvmt) | EditAction::JumpBackspace(mvmt) => {
                let to_delete = if self.selection.is_caret() {
                    movement(mvmt, self.selection, data, true)
                } else {
                    self.selection
                };
                data.edit(to_delete.range(), "");
                self.selection = Selection::caret(to_delete.min());
            }
            EditAction::Move(mvmt) => self.selection = movement(mvmt, self.selection, data, false),
            EditAction::ModifySelection(mvmt) => {
                self.selection = movement(mvmt, self.selection, data, true)
            }
            EditAction::Click(action) => {
                if action.mods.shift() {
                    self.selection.end = action.column;
                } else {
                    self.selection = Selection::caret(action.column);
                }
            }
            EditAction::Drag(action) => self.selection.end = action.column,
            _ => (),
        }
    }

    pub fn draw(&self, ctx: &mut PaintCtx, point: impl Into<Point>) {
        self.layout.draw(ctx, point)
    }

    /// Returns `true` if the data passed here has been changed externally,
    /// which means things like our selection state may be out of sync.
    ///
    /// This would only happen in the unlikely case that somebody else has mutated
    /// the data before us while handling an event; if this is the case we ignore
    /// the event, and our data will be updated in `update`.
    fn data_is_stale(&self, data: &T) -> bool {
        self.layout.text().map(|t| !t.same(data)).unwrap_or(true)
    }

    fn insert(&mut self, text: &str, data: &mut T) {
        // if we aren't multiline, we insert only up to the first newline
        let text = if self.multi_line {
            text
        } else {
            text.split('\n').next().unwrap_or("")
        };
        let sel = self.selection.range();
        data.edit(sel, text);
        self.selection = Selection::caret(self.selection.min() + text.len());
    }

    /// Delete to previous grapheme if in caret mode.
    /// Otherwise just delete everything inside the selection.
    fn delete_backward(&mut self, data: &mut T) {
        let cursor_pos = if self.selection.is_caret() {
            let del_end = self.selection.end;
            let del_start = offset_for_delete_backwards(&self.selection, data);
            data.edit(del_start..del_end, "");
            del_start
        } else {
            data.edit(self.selection.range(), "");
            self.selection.min()
        };

        self.selection = Selection::caret(cursor_pos);
    }

    fn delete_forward(&mut self, data: &mut T) {
        let to_delete = if self.selection.is_caret() {
            movement(Movement::Right, self.selection, data, false)
        } else {
            self.selection
        };

        data.edit(to_delete.range(), "");
        self.selection = Selection::caret(self.selection.min());
    }

    pub fn copy(&self, data: &mut T) {
        if !self.data_is_stale(data) {
            self.set_clipboard()
        }
    }

    pub fn cut(&mut self, data: &mut T) {
        if !self.data_is_stale(data) {
            self.set_clipboard();
            self.delete_backward(data);
        }
    }

    fn set_clipboard(&self) {
        if let Some(text) = self
            .layout
            .text()
            .and_then(|txt| txt.slice(self.selection.range()))
        {
            if !text.is_empty() {
                Application::global().clipboard().put_string(text);
            }
        }
    }

    pub fn paste(&mut self, t: String, data: &mut T) {
        self.do_edit(EditAction::Paste(t), data)
    }
}
