mod grep;

use grep::grep;

fn main() {
    let matches = grep().unwrap();
    for m in matches {
        println!("{:?}", m);
    }
}
