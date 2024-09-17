use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::{overrides::OverrideBuilder, types::TypesBuilder, WalkBuilder};

const PATTERN: &str = r"#\[derive\([^\)]+\)\]";

pub type Matches = HashMap<PathBuf, HashSet<usize>>;

struct Match {
    file_path: PathBuf,
    line_number: usize,
}

pub fn grep<P: AsRef<Path>>(root: P, exclude: Vec<String>) -> Result<Matches, String> {
    let (tx, rx) = mpsc::channel();

    let mut type_builder = TypesBuilder::new();
    type_builder.add_defaults().select("rust");

    let mut override_builder = OverrideBuilder::new(root.as_ref());
    for glob in exclude {
        override_builder
            .add(&format!("!{}", glob))
            .map_err(|e| e.to_string())?;
    }

    let walker = WalkBuilder::new(root.as_ref())
        .standard_filters(true)
        .types(type_builder.build().unwrap())
        .overrides(override_builder.build().unwrap())
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

    let matches: Result<Vec<Match>, String> = rx.into_iter().collect();

    matches.map(|ms| {
        ms.into_iter()
            .fold(HashMap::<PathBuf, HashSet<usize>>::new(), |mut acc, m| {
                acc.entry(m.file_path).or_default().insert(m.line_number);
                acc
            })
    })
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
            line_number: mat.line_number().unwrap() as usize,
        };
        self.tx.send(Ok(m)).unwrap();
        Ok(true)
    }
}

fn grep_file(path: PathBuf, tx: &Sender<Result<Match, String>>) {
    let matcher = RegexMatcherBuilder::new().build(PATTERN).unwrap();

    let mut searcher = SearcherBuilder::new().line_number(true).build();
    let sink = SearchSink {
        tx,
        file_path: &path,
    };
    if let Err(err) = searcher.search_path(&matcher, &path, sink) {
        tx.send(Err(err.to_string())).unwrap();
    }
}
