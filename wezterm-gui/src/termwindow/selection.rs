use crate::overlay::{CopyOverlay, QuickSelectOverlay, SearchOverlay};
use crate::selection::{Selection, SelectionCoordinate, SelectionMode, SelectionRange};
use crate::termwindow::BaseTermWindow;
use mux::pane::{Pane, PaneId};
use std::cell::RefMut;
use std::rc::Rc;
use wezterm_term::StableRowIndex;

pub trait Selectable: BaseTermWindow {
    fn selection(&self, pane_id: PaneId) -> RefMut<Selection>;

    fn selection_text(&self, pane: &Rc<dyn Pane>) -> String {
        let mut s = String::new();
        if let Some(sel) = self
            .selection(pane.pane_id())
            .range
            .as_ref()
            .map(|r| r.normalize())
        {
            let mut last_was_wrapped = false;
            let first_row = sel.rows().start;
            let last_row = sel.rows().end;

            for line in pane.get_logical_lines(sel.rows()) {
                if !s.is_empty() && !last_was_wrapped {
                    s.push('\n');
                }
                let last_idx = line.physical_lines.len().saturating_sub(1);
                for (idx, phys) in line.physical_lines.iter().enumerate() {
                    let this_row = line.first_row + idx as StableRowIndex;
                    if this_row >= first_row && this_row < last_row {
                        let last_phys_idx = phys.cells().len().saturating_sub(1);
                        let cols = sel.cols_for_row(this_row);
                        let last_col_idx = cols.end.saturating_sub(1).min(last_phys_idx);
                        let col_span = phys.columns_as_str(cols);
                        // Only trim trailing whitespace if we are the last line
                        // in a wrapped sequence
                        if idx == last_idx {
                            s.push_str(col_span.trim_end());
                        } else {
                            s.push_str(&col_span);
                        }

                        last_was_wrapped = last_col_idx == last_phys_idx
                            && phys
                                .cells()
                                .get(last_col_idx)
                                .map(|c| c.attrs().wrapped())
                                .unwrap_or(false);
                    }
                }
            }
        }

        s
    }

    fn extend_selection_at_mouse_cursor(
        &mut self,
        mode: Option<SelectionMode>,
        pane: &Rc<dyn Pane>,
    ) {
        self.selection(pane.pane_id()).seqno = pane.get_current_seqno();
        let mode = mode.unwrap_or(SelectionMode::Cell);
        let (x, y, position) = match self.pane_state(pane.pane_id()).mouse_terminal_coords {
            Some(coords) => (coords.0.column, coords.1, coords.0),
            None => return,
        };
        match mode {
            SelectionMode::Cell => {
                let origin = self
                    .selection(pane.pane_id())
                    .origin
                    .unwrap_or(SelectionCoordinate { x, y });
                self.selection(pane.pane_id()).origin = Some(origin);

                let (start_x, end_x) = if (origin.x <= x && origin.y == y) || origin.y < y {
                    (origin.x, x.saturating_sub(1))
                } else {
                    (origin.x.saturating_sub(1), x)
                };

                self.selection(pane.pane_id()).range = if origin.x != x || origin.y != y {
                    Some(
                        SelectionRange::start(SelectionCoordinate {
                            x: start_x,
                            y: origin.y,
                        })
                        .extend(SelectionCoordinate { x: end_x, y }),
                    )
                } else {
                    None
                };
            }
            SelectionMode::Word => {
                let end_word = SelectionRange::word_around(SelectionCoordinate { x, y }, &**pane);

                let start_coord = self
                    .selection(pane.pane_id())
                    .origin
                    .clone()
                    .unwrap_or(end_word.start);
                let start_word = SelectionRange::word_around(start_coord, &**pane);

                let selection_range = start_word.extend_with(end_word);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
            SelectionMode::Line => {
                let end_line = SelectionRange::line_around(SelectionCoordinate { x, y }, &**pane);

                let start_coord = self
                    .selection(pane.pane_id())
                    .origin
                    .clone()
                    .unwrap_or(end_line.start);
                let start_line = SelectionRange::line_around(start_coord, &**pane);

                let selection_range = start_line.extend_with(end_line);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
            SelectionMode::SemanticZone => {
                let end_word = SelectionRange::zone_around(SelectionCoordinate { x, y }, &**pane);

                let start_coord = self
                    .selection(pane.pane_id())
                    .origin
                    .clone()
                    .unwrap_or(end_word.start);
                let start_word = SelectionRange::zone_around(start_coord, &**pane);

                let selection_range = start_word.extend_with(end_word);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
        }

        let dims = pane.get_dimensions();

        // Scroll viewport when mouse mouves out of its vertical bounds
        if position.row == 0 && position.y_pixel_offset < 0 {
            self.set_viewport(pane.pane_id(), Some(y.saturating_sub(1)), dims);
        } else if position.row >= dims.viewport_rows as i64 {
            let top = self
                .get_viewport(pane.pane_id())
                .unwrap_or(dims.physical_top);
            self.set_viewport(pane.pane_id(), Some(top + 1), dims);
        }

        self.invalidate_window();
    }

    fn select_text_at_mouse_cursor(&mut self, mode: SelectionMode, pane: &Rc<dyn Pane>) {
        let (x, y) = match self.pane_state(pane.pane_id()).mouse_terminal_coords {
            Some(coords) => (coords.0.column, coords.1),
            None => return,
        };
        match mode {
            SelectionMode::Line => {
                let start = SelectionCoordinate { x, y };
                let selection_range = SelectionRange::line_around(start, &**pane);

                self.selection(pane.pane_id()).origin = Some(start);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
            SelectionMode::Word => {
                let selection_range =
                    SelectionRange::word_around(SelectionCoordinate { x, y }, &**pane);

                self.selection(pane.pane_id()).origin = Some(selection_range.start);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
            SelectionMode::SemanticZone => {
                let selection_range =
                    SelectionRange::zone_around(SelectionCoordinate { x, y }, &**pane);

                self.selection(pane.pane_id()).origin = Some(selection_range.start);
                self.selection(pane.pane_id()).range = Some(selection_range);
            }
            SelectionMode::Cell => {
                self.selection(pane.pane_id())
                    .begin(SelectionCoordinate { x, y });
            }
        }

        self.selection(pane.pane_id()).seqno = pane.get_current_seqno();
        self.invalidate_window();
    }

    fn check_for_dirty_lines_and_invalidate_selection(&mut self, pane: &Rc<dyn Pane>) {
        let dims = pane.get_dimensions();
        let viewport = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top);
        let visible_range = viewport..viewport + dims.viewport_rows as StableRowIndex;
        let seqno = self.selection(pane.pane_id()).seqno;
        let dirty = pane.get_changed_since(visible_range, seqno);

        if dirty.is_empty() {
            return;
        }
        if pane.downcast_ref::<SearchOverlay>().is_none()
            && pane.downcast_ref::<CopyOverlay>().is_none()
            && pane.downcast_ref::<QuickSelectOverlay>().is_none()
        {
            // If any of the changed lines intersect with the
            // selection, then we need to clear the selection, but not
            // when the search overlay is active; the search overlay
            // marks lines as dirty to force invalidate them for
            // highlighting purpose but also manipulates the selection
            // and we want to allow it to retain the selection it made!

            let clear_selection =
                if let Some(selection_range) = self.selection(pane.pane_id()).range.as_ref() {
                    let selection_rows = selection_range.rows();
                    selection_rows.into_iter().any(|row| dirty.contains(row))
                } else {
                    false
                };

            if clear_selection {
                self.selection(pane.pane_id()).range.take();
                self.selection(pane.pane_id()).origin.take();
                self.selection(pane.pane_id()).seqno = pane.get_current_seqno();
            }
        }
    }
}

impl Selectable for super::TermWindow {
    fn selection(&self, pane_id: PaneId) -> RefMut<Selection> {
        RefMut::map(self.pane_state(pane_id), |state| &mut state.selection)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::termwindow::PaneState;
    use mux::domain::DomainId;
    use mux::renderable::{RenderableDimensions, StableCursorPosition};
    use portable_pty::PtySize;
    use rangeset::RangeSet;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ops::Range;
    use termwiz::surface::SequenceNo;
    use url::Url;
    use wezterm_term::color::ColorPalette;
    use wezterm_term::{CellAttributes, ClickPosition, KeyCode, KeyModifiers, Line, MouseEvent};

    struct TestTermWindow {
        pane_state: RefCell<HashMap<PaneId, PaneState>>,
    }

    impl BaseTermWindow for TestTermWindow {
        fn invalidate_window(&self) {}

        fn pane_state(&self, pane_id: PaneId) -> RefMut<PaneState> {
            RefMut::map(self.pane_state.borrow_mut(), |state| {
                state.entry(pane_id).or_insert_with(PaneState::default)
            })
        }
    }

    impl Selectable for TestTermWindow {
        fn selection(&self, pane_id: PaneId) -> RefMut<Selection> {
            RefMut::map(self.pane_state(pane_id), |state| &mut state.selection)
        }
    }

    struct FakePane {
        lines: Vec<Line>,
    }

    impl Pane for FakePane {
        fn pane_id(&self) -> PaneId {
            0
        }
        fn get_cursor_position(&self) -> StableCursorPosition {
            unimplemented!()
        }
        fn get_current_seqno(&self) -> SequenceNo {
            0
        }
        fn get_changed_since(
            &self,
            _: Range<StableRowIndex>,
            _: SequenceNo,
        ) -> RangeSet<StableRowIndex> {
            unimplemented!()
        }
        fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
            let first = lines.start;
            (
                first,
                self.lines
                    .iter()
                    .skip(lines.start as usize)
                    .take((lines.end - lines.start) as usize)
                    .cloned()
                    .collect(),
            )
        }
        fn get_dimensions(&self) -> RenderableDimensions {
            RenderableDimensions {
                cols: 40,
                viewport_rows: 3,
                scrollback_rows: 7,
                physical_top: 4,
                scrollback_top: 0,
            }
        }
        fn get_title(&self) -> String {
            unimplemented!()
        }
        fn send_paste(&self, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
            Ok(None)
        }
        fn writer(&self) -> RefMut<dyn std::io::Write> {
            unimplemented!()
        }
        fn resize(&self, _: PtySize) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn mouse_event(&self, _: MouseEvent) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn is_dead(&self) -> bool {
            unimplemented!()
        }
        fn palette(&self) -> ColorPalette {
            unimplemented!()
        }
        fn domain_id(&self) -> DomainId {
            unimplemented!()
        }
        fn is_mouse_grabbed(&self) -> bool {
            false
        }
        fn is_alt_screen_active(&self) -> bool {
            false
        }
        fn get_current_working_dir(&self) -> Option<Url> {
            None
        }
        fn key_down(&self, _: KeyCode, _: KeyModifiers) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn key_up(&self, _: KeyCode, _: KeyModifiers) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    fn test_pane() -> Rc<dyn Pane> {
        Rc::new(FakePane {
            lines: vec![
                Line::from_text(
                    "0000000000000000000000000000000000000000",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "LwqSxlBaoRF8ikBQ4n9roGxXoku6FITVfBy0tfIe",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "wez term and rust rocks wYy7d7cz4AnB4a4s",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "HHTci_+_jnLNwkIV3SrWzthAA1ZZVWQHpA8NTP0t",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "n6yuizjKsSOA4LyCWmkMR_+_D9amfXWjglQEsFth",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "btKm94BK9f1KJJgHq67TSrTXM4UPcgLttCxODHrI",
                    &CellAttributes::default(),
                    0,
                ),
                Line::from_text(
                    "1111111111111111111111111111111111111111",
                    &CellAttributes::default(),
                    0,
                ),
            ],
        })
    }

    fn terminal_coords(
        column: usize,
        row: i64,
        buffer_row: isize,
    ) -> Option<(ClickPosition, StableRowIndex)> {
        Some((
            ClickPosition {
                column,
                row,
                x_pixel_offset: 0,
                y_pixel_offset: 0,
            },
            buffer_row,
        ))
    }

    #[test]
    fn test_selection_cell_no_selection_on_same_cell() {
        let mut termwindow = TestTermWindow {
            pane_state: RefCell::new(HashMap::new()),
        };

        let pane = test_pane();

        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(6, 1, 3);
        termwindow.select_text_at_mouse_cursor(SelectionMode::Cell, &pane);

        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(6, 1, 3);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        assert!(
            termwindow.selection(pane.pane_id()).range.is_none(),
            "selection needs to be none"
        );
        assert_eq!(termwindow.selection_text(&pane), "", "wrong selection text");
    }

    #[test]
    fn test_selection_cell_same_line() {
        let mut termwindow = TestTermWindow {
            pane_state: RefCell::new(HashMap::new()),
        };

        let pane = test_pane();

        // set selection origin
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(6, 1, 3);
        termwindow.select_text_at_mouse_cursor(SelectionMode::Cell, &pane);

        // select after cursor
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(11, 1, 3);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 6, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 10, y: 3 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "+_jnL",
            "wrong selection text"
        );

        // select before cursor
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(1, 1, 3);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 5, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 1, y: 3 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "HTci_",
            "wrong selection text"
        );
    }

    #[test]
    fn test_selection_cell_next_lines() {
        let mut termwindow = TestTermWindow {
            pane_state: RefCell::new(HashMap::new()),
        };

        let pane = test_pane();

        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(6, 1, 3);
        termwindow.select_text_at_mouse_cursor(SelectionMode::Cell, &pane);

        // select after cursor x
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(11, 1, 5);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 6, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 10, y: 5 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "+_jnLNwkIV3SrWzthAA1ZZVWQHpA8NTP0t\nn6yuizjKsSOA4LyCWmkMR_+_D9amfXWjglQEsFth\nbtKm94BK9f1",
            "wrong selection text"
        );

        // select before cursor x
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(1, 1, 5);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 6, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 0, y: 5 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "+_jnLNwkIV3SrWzthAA1ZZVWQHpA8NTP0t\nn6yuizjKsSOA4LyCWmkMR_+_D9amfXWjglQEsFth\nb",
            "wrong selection text"
        );
    }

    #[test]
    fn test_selection_cell_previous_lines() {
        let mut termwindow = TestTermWindow {
            pane_state: RefCell::new(HashMap::new()),
        };

        let pane = test_pane();

        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(6, 1, 3);
        termwindow.select_text_at_mouse_cursor(SelectionMode::Cell, &pane);

        // select after cursor x
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(11, 1, 1);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 5, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 11, y: 1 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "8ikBQ4n9roGxXoku6FITVfBy0tfIe\nwez term and rust rocks wYy7d7cz4AnB4a4s\nHHTci_",
            "wrong selection text"
        );

        // select before cursor x
        termwindow.pane_state(pane.pane_id()).mouse_terminal_coords = terminal_coords(1, 1, 1);
        termwindow.extend_selection_at_mouse_cursor(Some(SelectionMode::Cell), &pane);

        let SelectionRange { start, end } = termwindow
            .selection(pane.pane_id())
            .range
            .expect("selection was none");

        assert_eq!(
            start,
            SelectionCoordinate { x: 5, y: 3 },
            "wrong start coords"
        );
        assert_eq!(end, SelectionCoordinate { x: 1, y: 1 }, "wrong end coords");
        assert_eq!(
            termwindow.selection_text(&pane),
            "wqSxlBaoRF8ikBQ4n9roGxXoku6FITVfBy0tfIe\nwez term and rust rocks wYy7d7cz4AnB4a4s\nHHTci_",
            "wrong selection text"
        );
    }
}
