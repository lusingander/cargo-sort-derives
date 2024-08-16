mod ext;
mod grep;
mod sort;

use grep::grep;
use sort::process_file;

fn main() {
    for (file_path, line_numbers) in grep().unwrap() {
        process_file(&file_path, line_numbers).unwrap();
    }
}
