use std::io::BufRead;

pub trait BufReadExt: BufRead {
    fn lines_with_terminator(self) -> LinesWithTerminator<Self>
    where
        Self: Sized,
    {
        LinesWithTerminator { reader: self }
    }
}

impl<R: BufRead> BufReadExt for R {}

pub struct LinesWithTerminator<R> {
    reader: R,
}

impl<R: BufRead> Iterator for LinesWithTerminator<R> {
    type Item = std::io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buf)),
            Err(e) => Some(Err(e)),
        }
    }
}
