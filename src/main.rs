mod grep;

use grep::grep;

fn main() {
    let matches = grep().unwrap();

    for (file_path, line_numbers) in matches {
        for line_number in &line_numbers {
            println!("{}:{}", file_path.display(), line_number);
        }
    }
}
