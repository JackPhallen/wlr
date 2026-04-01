use std::collections::VecDeque;
use std::io::{self, Write};

pub struct FilterConfig {
    pub separator: Vec<u8>,
    pub before: usize,
    pub after: usize,
}

pub struct LineEmitter {
    separator: Vec<u8>,
    before: usize,
    after: usize,
    before_buffer: Option<VecDeque<Vec<u8>>>,
    gap_len: usize,
    emitted_block: bool,
    after_remaining: usize,
}

impl LineEmitter {
    /// Create the line-level emission state.
    ///
    /// This layer is intentionally ignorant of ANSI parsing and colors. It only
    /// knows whether each completed line matched and how much surrounding
    /// context should be emitted around matches.
    pub fn new(config: FilterConfig) -> Self {
        let before_buffer = if config.before > 0 {
            Some(VecDeque::with_capacity(config.before))
        } else {
            None
        };

        Self {
            separator: config.separator,
            before: config.before,
            after: config.after,
            before_buffer,
            gap_len: 0,
            emitted_block: false,
            after_remaining: 0,
        }
    }

    /// Consume one completed line and decide whether to emit it now, keep it as
    /// possible `before` context, or treat it as a gap between matching blocks.
    pub fn finish_line<W: Write>(
        &mut self,
        raw: Vec<u8>,
        matches: bool,
        out: &mut W,
    ) -> io::Result<()> {
        if matches {
            self.emit_match(raw, out)?;
        } else {
            self.finish_non_matching(raw, out)?;
        }

        Ok(())
    }

    /// Emit a matching line.
    ///
    /// This may do three things in order:
    /// 1. insert a separator if the previous emitted block is truly disjoint
    /// 2. flush any buffered `before` context that has not already been emitted
    /// 3. write the matching line itself
    ///
    /// After a match, the trailing `after` window is refreshed.
    fn emit_match<W: Write>(&mut self, raw: Vec<u8>, out: &mut W) -> io::Result<()> {
        let gap_len = self.gap_len;
        let overlap = self.has_overlap(gap_len);

        if self.emitted_block && gap_len > 0 && !overlap {
            out.write_all(&self.separator)?;
        }

        self.emit_pending_before_context(gap_len, out)?;
        out.write_all(&raw)?;

        self.gap_len = 0;
        self.after_remaining = self.after;
        self.emitted_block = true;
        Ok(())
    }

    /// Process a non-matching line.
    ///
    /// Non-matching lines serve two possible roles:
    /// - future `before` context for a later match
    /// - current `after` context for a recent match
    ///
    /// If neither applies, the line is only counted as part of the gap between
    /// emitted blocks.
    fn finish_non_matching<W: Write>(&mut self, raw: Vec<u8>, out: &mut W) -> io::Result<()> {
        if self.emitted_block {
            self.gap_len += 1;
        }

        self.push_before_line(raw.clone());

        if self.after_remaining > 0 {
            out.write_all(&raw)?;
            self.after_remaining -= 1;
        }

        Ok(())
    }

    /// Emit buffered `before` lines that belong to the new match but have not
    /// already been printed as overlapping `after` context from the previous
    /// block.
    ///
    /// This is the key piece that lets `-B/-A` windows merge cleanly instead of
    /// duplicating shared context lines.
    fn emit_pending_before_context<W: Write>(
        &mut self,
        gap_len: usize,
        out: &mut W,
    ) -> io::Result<()> {
        let Some(buffer) = self.before_buffer.as_mut() else {
            return Ok(());
        };

        let buffered_len = buffer.len();
        if buffered_len == 0 {
            return Ok(());
        }

        let start_pos = gap_len.saturating_sub(buffered_len) + 1;
        let already_emitted = gap_len.min(self.after);
        let first_needed_pos = gap_len.saturating_sub(self.before).saturating_add(1);
        let first_unemitted_pos = already_emitted + 1;
        let emit_from_pos = first_needed_pos.max(first_unemitted_pos);

        if emit_from_pos > gap_len {
            return Ok(());
        }

        let skip = emit_from_pos.saturating_sub(start_pos);
        for line in buffer.iter().skip(skip) {
            out.write_all(line)?;
        }

        Ok(())
    }

    /// Push a completed non-matching line into the bounded `before` history.
    ///
    /// When `before == 0`, this is a no-op so we avoid maintaining any rolling
    /// history in the common no-context case.
    fn push_before_line(&mut self, raw: Vec<u8>) {
        let Some(buffer) = self.before_buffer.as_mut() else {
            return;
        };

        if buffer.len() == self.before {
            buffer.pop_front();
        }
        buffer.push_back(raw);
    }

    /// Report whether the gap between two matching lines is small enough that
    /// the old block's `after` context and the new block's `before` context
    /// should merge into one continuous emitted section.
    fn has_overlap(&self, gap_len: usize) -> bool {
        self.before > 0
            && self.after > 0
            && gap_len > 0
            && gap_len <= self.before + self.after - 1
    }
}
