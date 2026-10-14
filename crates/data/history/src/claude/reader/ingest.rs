//! Reads bounded transcript chunks and retains incomplete lines.

use super::{IngestResult, PartialRead, READ_CHUNK_BYTES, Reader, TranscriptFile};
use quotaline_claude::claude_transcripts;
use quotaline_claude::claude_transcripts::PartialLine;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    sync::Arc,
    time::Instant,
};

impl Reader {
    pub(super) fn ingest(
        &mut self,
        file: &TranscriptFile,
        cutoff: f64,
        byte_limit: usize,
        deadline: Instant,
        retains_incomplete_tail: bool,
    ) -> IngestResult {
        let key = file.path.to_string_lossy().into_owned();
        let offset = self.state.offset(&key);
        let stored_offset = offset.map_or(0, |offset| offset.bytes);
        let generation = self.scan.as_ref().map_or_else(
            || offset.map_or(self.scan_generation, |offset| offset.scan_generation),
            |scan| scan.generation,
        );
        let mut partial = self.partial_reads.remove(&key).unwrap_or(PartialRead {
            committed_offset: stored_offset,
            cursor: stored_offset,
            pending: Vec::new(),
            oversized: false,
        });
        if file.size < partial.cursor {
            partial = PartialRead {
                committed_offset: 0,
                cursor: 0,
                pending: Vec::new(),
                oversized: false,
            };
            Arc::make_mut(&mut self.state).set_offset(key.clone(), 0, generation);
        }
        if partial.cursor >= file.size {
            return IngestResult {
                changed: partial.committed_offset != stored_offset,
                bytes_read: 0,
                complete: true,
            };
        }
        let Ok(mut handle) = File::open(&file.path) else {
            return IngestResult::failed();
        };
        self.workload.files_opened += 1;
        if handle.seek(SeekFrom::Start(partial.cursor)).is_err() {
            return IngestResult::failed();
        }
        let mut bytes_read = 0;
        let mut changed = false;
        let mut stale_eof = false;
        let project = claude_transcripts::project_name(&file.path, &self.root);
        while partial.cursor < file.size && bytes_read < byte_limit && Instant::now() < deadline {
            let count = READ_CHUNK_BYTES
                .min(byte_limit - bytes_read)
                .min((file.size - partial.cursor) as usize);
            let mut chunk = vec![0; count];
            let Ok(size) = handle.read(&mut chunk) else {
                return IngestResult::failed();
            };
            if size == 0 {
                stale_eof = true;
                break;
            }
            chunk.truncate(size);
            let chunk_start = partial.cursor;
            partial.cursor += size as u64;
            bytes_read += size;
            let (next, records) = claude_transcripts::consume_chunk(
                PartialLine {
                    committed_offset: partial.committed_offset,
                    pending: partial.pending,
                    oversized: partial.oversized,
                },
                &key,
                &chunk,
                chunk_start,
                self.config.maximum_line_bytes,
                cutoff,
            );
            partial.committed_offset = next.committed_offset;
            partial.pending = next.pending;
            partial.oversized = next.oversized;
            for record in &records {
                changed |= Arc::make_mut(&mut self.state).ingest(record, &project);
            }
        }
        Arc::make_mut(&mut self.state).set_offset(key.clone(), partial.committed_offset, generation);
        let changed = changed || partial.committed_offset != stored_offset;
        let complete = stale_eof || partial.cursor >= file.size;
        if !stale_eof
            && !(partial.pending.is_empty() && !partial.oversized && partial.cursor >= file.size)
            && !(partial.cursor >= file.size && !retains_incomplete_tail)
        {
            self.retain_partial(partial, key);
        }
        self.record_partial_workload();
        self.workload.bytes_read += bytes_read as u64;
        IngestResult {
            changed,
            bytes_read,
            complete,
        }
    }

    fn retain_partial(&mut self, partial: PartialRead, key: String) {
        self.partial_reads.insert(key.clone(), partial);
        let mut retained = self
            .partial_reads
            .values()
            .map(|partial| partial.pending.len())
            .sum::<usize>();
        while retained > self.config.maximum_retained_partial_bytes {
            let victim = self
                .partial_reads
                .iter()
                .filter(|(candidate, _)| *candidate != &key)
                .max_by(|(left_key, left), (right_key, right)| {
                    left.pending
                        .len()
                        .cmp(&right.pending.len())
                        .then_with(|| left_key.cmp(right_key))
                })
                .map(|(candidate, _)| candidate.clone())
                .expect("one partial tail fits the retention budget");
            retained -= self.partial_reads.remove(&victim).unwrap().pending.len();
        }
    }
}
