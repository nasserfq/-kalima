use super::doc_state::{MAX_UNDO, Selection, UNDO_LIST_MAX, Undo, next_math_session};
use super::{Arc, Document, Session};

impl Session {
    /// Snapshot for undo before a change. Returns whether it took a step: typing that is
    /// already in an open step is not a new one.
    pub fn checkpoint(&mut self, label: &str) -> bool {
        if label == "Typing" && self.typing_open {
            return false;
        }
        self.typing_open = label == "Typing";
        self.history.push(Arc::new(Undo {
            label: label.to_string(),
            doc: self.doc.clone(),
            sel: self.sel.clone(),
            pending: self.pending.clone(),
            column: self.column.clone(),
        }));
        if self.history.len() > MAX_UNDO {
            self.history.remove(0);
            self.undo_evicted += 1;
        }
        self.redo.clear();
        // The newest step is what a joined command merges into.
        self.join_anchor = true;
        true
    }

    /// Record `doc`/`sel` as their own undo step after the fact and close the typing group, so
    /// Undo right after an automatic change (AutoFormat, AutoCorrect) reverts only that change.
    pub fn push_undo(&mut self, label: &str, doc: Document, sel: Selection) {
        self.history.push(Arc::new(Undo { label: label.to_string(), doc, sel, pending: self.pending.clone(), column: None }));
        if self.history.len() > MAX_UNDO {
            self.history.remove(0);
            self.undo_evicted += 1;
        }
        self.typing_open = false;
        self.join_anchor = true;
    }

    /// Make the next mutating command part of the previous undo step instead of a new one, so a
    /// drag that runs a command every frame is a single Undo. Call it on every frame of the drag
    /// except the first; it is consumed by the next `run`.
    pub fn join_next_undo(&mut self) {
        self.join_next = true;
    }

    /// Make the next equation typing command (`equation.type` or `equation.backspace`) join the
    /// previous equation deletion's undo step. Unlike [`Self::join_next_undo`], the promise is
    /// not generic: any other command consumes and discards it, so unrelated edits get their own
    /// checkpoint. A deletion-only IME request therefore cannot leak a join into the next edit.
    pub fn join_next_equation_edit(&mut self) {
        self.equation_join_next = true;
    }

    /// Discard an equation replacement join that received no replacement (for example an empty
    /// commit), so it cannot join a later composition's typing.
    pub fn clear_equation_join(&mut self) {
        self.equation_join_next = false;
    }

    /// Make the next `text.insert` at the current selection join a successful body
    /// `DeleteSurrounding`'s undo step. Unlike [`Self::join_next_undo`], the promise is scoped to
    /// that exact replacement: any other command consumes and discards it, so an intervening
    /// ordinary edit gets its own checkpoint.
    pub fn join_next_ime_text_replacement(&mut self) {
        self.ime_text_join = Some(self.sel.clone());
    }

    /// Discard a body replacement join that received no replacement (an empty or refused
    /// commit, a new request or a refusal), so it cannot join a later composition.
    pub fn clear_ime_text_join(&mut self) {
        self.ime_text_join = None;
    }

    /// Discard every pending IME replacement join. A refused surrounding request supersedes the
    /// composition it interrupts, including any promise from an earlier accepted request.
    pub fn clear_ime_joins(&mut self) {
        self.equation_join_next = false;
        self.ime_text_join = None;
    }

    /// Stamp the document's save metadata (modified time, last modified by, revision; created
    /// when it has none) the way Word does on every save. `file.save` calls it, and so does any
    /// front end that writes the bytes itself (the web build's download, #262). Like Word, the
    /// document also keeps the zoom it is shown at (#995).
    pub fn stamp_save(&mut self) {
        self.doc.settings.zoom = crate::cmd::view::saved_zoom(&self.view);
        let core = &mut self.doc.core;
        core.modified = crate::cmd::now_iso();
        if core.created.is_empty() {
            core.created = core.modified.clone();
        }
        core.last_modified_by = self.author.clone();
        core.revision = core.revision.saturating_add(1);
    }

    /// Whether typing is still collecting into one undo step (no other command since).
    pub fn typing_open(&self) -> bool {
        self.typing_open
    }

    /// Reopen the typing group when the last undo step is typing, so the keys typed next join
    /// it: after an input method's rewrite was joined to it (`join_next_undo` closes the group).
    pub fn continue_typing(&mut self) {
        if self.undo_label() == Some("Typing") {
            self.typing_open = true;
        }
    }

    /// Close an open typing group (caret moved, other command).
    pub fn close_typing(&mut self) {
        self.typing_open = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.history.last().map(|u| u.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|u| u.label.as_str())
    }

    /// Undo steps pushed so far, evicted ones included: only grows while steps are added.
    pub(crate) fn undo_steps_so_far(&self) -> u64 {
        self.history.len() as u64 + self.undo_evicted
    }

    /// The number of undo steps.
    pub fn undo_depth(&self) -> usize {
        self.history.len()
    }

    /// Where the undo history stands, for [`Session::merge_undo_since`].
    pub fn undo_mark(&self) -> u64 {
        (self.history.len() as u64).saturating_add(self.undo_evicted)
    }

    /// Makes the steps recorded since `mark` one undo step, the first: a drag that applied its
    /// value as it went (a slider in a pane) undoes in one go.
    pub fn merge_undo_since(&mut self, mark: u64) {
        let keep = mark.saturating_sub(self.undo_evicted).saturating_add(1);
        self.history.truncate(usize::try_from(keep).unwrap_or(usize::MAX));
    }

    pub fn undo_labels(&self) -> Vec<String> {
        self.history.iter().rev().map(|u| u.label.clone()).collect()
    }

    /// The Undo button's list (#701): the undo steps' labels (English, as commands name them),
    /// newest first, at most [`UNDO_LIST_MAX`].
    pub fn undo_history(&self) -> Vec<&str> {
        self.history.iter().rev().take(UNDO_LIST_MAX).map(|u| u.label.as_str()).collect()
    }

    /// Undo the newest `count` steps in one go, as picking an entry in the Undo list does (#701).
    /// More than the undo depth undoes everything; a step that would take the document's
    /// protection away stops it (only Stop Protection lifts it). Returns the steps undone.
    pub fn undo_steps(&mut self, count: usize) -> usize {
        let mut done = 0;
        while done < count && !self.step_lifts_protection(false) && self.undo() {
            done += 1;
        }
        done
    }

    /// Undoes the newest step: closes typing, records an AutoCorrect exception when the undone step
    /// was its automatic capital, swaps the document, selection, pending formatting and column with
    /// the redo snapshot, replaces the body identity, refreshes equation editing and touches layout.
    pub fn undo(&mut self) -> bool {
        self.typing_open = false;
        // Undoing the automatic capitalisation's own step makes its word an exception; any
        // other Undo forgets the capital.
        let steps = self.history.len() as u64 + self.undo_evicted;
        if let Some(c) = self.autocap.take().filter(|c| c.depth == steps) {
            c.add_exceptions(&mut self.prefs.autocorrect);
        }
        let Some(u) = self.history.pop().map(Arc::unwrap_or_clone) else { return false };
        let cur = Undo {
            label: u.label.clone(),
            doc: std::mem::replace(&mut self.doc, u.doc),
            sel: std::mem::replace(&mut self.sel, u.sel),
            pending: std::mem::replace(&mut self.pending, u.pending),
            column: std::mem::replace(&mut self.column, u.column),
        };
        self.redo.push(Arc::new(cur));
        // Undo swaps the whole body back: a composition belongs to the text it showed.
        self.note_body_replaced();
        self.refresh_math_session();
        self.touch();
        true
    }

    /// Would Undo (or Redo) take the document's protection away? Only Stop Protection lifts it.
    pub fn step_lifts_protection(&self, redo: bool) -> bool {
        let next = if redo { self.redo.last() } else { self.history.last() };
        next.is_some_and(|u| crate::cmd::protect::protection_lifted(&self.doc, &u.doc))
    }

    /// Redoes the newest undone step by swapping the snapshots back, replacing the body identity,
    /// refreshing equation editing and touching layout.
    pub fn redo(&mut self) -> bool {
        self.typing_open = false;
        let Some(u) = self.redo.pop().map(Arc::unwrap_or_clone) else { return false };
        let cur = Undo {
            label: u.label.clone(),
            doc: std::mem::replace(&mut self.doc, u.doc),
            sel: std::mem::replace(&mut self.sel, u.sel),
            pending: std::mem::replace(&mut self.pending, u.pending),
            column: std::mem::replace(&mut self.column, u.column),
        };
        self.history.push(Arc::new(cur));
        self.note_body_replaced();
        self.refresh_math_session();
        self.touch();
        true
    }

    /// An equation's editing session cannot ride across Undo/Redo: the equation tree, or the
    /// object at that offset, may be a different one now. A new session token starts a new IME
    /// context; if tokens are exhausted, the equation is left (fail closed).
    fn refresh_math_session(&mut self) {
        if self.math.is_none() {
            return;
        }
        match next_math_session() {
            Some(session) => {
                if let Some(m) = self.math.as_mut() {
                    m.session = session;
                }
            }
            None => self.math = None,
        }
    }

    /// Drop history (after open / new).
    pub fn reset_history(&mut self) {
        self.autocap = None;
        self.history.clear();
        self.redo.clear();
        self.typing_open = false;
        self.join_anchor = false;
    }

    /// The version snapshots belong to the document they were taken from: keeping the old ones
    /// would let a restore put the previous document's content into this one (#1327 follow-up).
    pub fn clear_versions(&mut self) {
        self.versions.clear();
    }

    /// Like [`Self::merge_undo_since`], and the merged step is called `label`: one Undo takes back
    /// everything an AI agent did in a turn, and the Undo button names it. Returns whether there was
    /// a step to name.
    pub fn squash_undo_since(&mut self, mark: u64, label: &str) -> bool {
        let start = usize::try_from(mark.saturating_sub(self.undo_evicted)).unwrap_or(usize::MAX);
        let Some(first) = self.history.get(start) else { return false };
        // The first step since the mark holds the document as it was before them all.
        let mut step = Undo::clone(first);
        step.label = label.to_string();
        self.history.truncate(start);
        self.history.push(Arc::new(step));
        self.typing_open = false;
        self.join_anchor = true;
        true
    }
}

#[cfg(test)]
mod squash_tests {
    use serde_json::json;
    use wordcraft_doc::StoryRef;

    use super::Session;

    /// Everything run after the mark comes back with one Undo, and what came before stays.
    #[test]
    fn one_undo_takes_back_a_whole_agent_turn() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        s.run("text.insert", &json!({"text": "Before. "})).unwrap();
        s.close_typing();
        let mark = s.undo_mark();
        assert!(!s.squash_undo_since(mark, "Agent"), "nothing to squash yet");
        s.run("text.insert", &json!({"text": "One. "})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "Two."})).unwrap();
        s.run("select.all", &json!({})).unwrap();
        s.run("format.bold", &json!({})).unwrap();
        assert!(s.squash_undo_since(mark, "Agent"));
        assert_eq!(s.undo_label(), Some("Agent"));
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc.plain_text(StoryRef::Body).trim(), "Before.");
        assert!(s.can_undo(), "the step before the mark is still there");
        s.run("edit.redo", &json!({})).unwrap();
        assert!(s.doc.plain_text(StoryRef::Body).contains("Two."), "and Redo brings the turn back");
        // A mark from further back than the history goes is harmless.
        assert!(!s.squash_undo_since(u64::MAX, "Agent"));
    }
}
