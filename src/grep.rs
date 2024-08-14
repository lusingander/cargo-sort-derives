use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::{types::TypesBuilder, WalkBuilder};

const PATTERN: &str = "#[derive(";

#[derive(Debug)]
pub struct Match {
    pub file_path: PathBuf,
    pub line_number: usize,
}

pub fn grep() -> Result<Vec<Match>, String> {
    let (tx, rx) = mpsc::channel();

    let walker = WalkBuilder::new(".")
        .standard_filters(true)
        .types(
            TypesBuilder::new()
                .add_defaults()
                .select("rust")
                .build()
                .unwrap(),
        )
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        grep_file(entry.into_path(), &tx);
                    }
                }
                ignore::WalkState::Continue
            }
            Err(err) => {
                tx.send(Err(err.to_string())).unwrap();
                ignore::WalkState::Quit
            }
        })
    });

    drop(tx);
    rx.into_iter().collect()
}

struct SearchSink<'a> {
    tx: &'a Sender<Result<Match, String>>,
    file_path: &'a Path,
}

impl<'a> Sink for SearchSink<'a> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let m = Match {
            file_path: self.file_path.to_owned(),
            line_number: mat.line_number().unwrap_or(0) as usize,
        };
        self.tx.send(Ok(m)).unwrap();
        Ok(true)
    }
}

fn grep_file(path: PathBuf, tx: &Sender<Result<Match, String>>) {
    let matcher = RegexMatcherBuilder::new()
        .fixed_strings(true)
        .build(PATTERN)
        .unwrap();

    let mut searcher = SearcherBuilder::new().line_number(true).build();
    let sink = SearchSink {
        tx,
        file_path: &path,
    };
    if let Err(err) = searcher.search_path(&matcher, &path, sink) {
        tx.send(Err(err.to_string())).unwrap();
    }
}
